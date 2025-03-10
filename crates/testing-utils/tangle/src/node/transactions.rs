use alloy_primitives::Address;
use alloy_provider::network::AnyNetwork;
use alloy_provider::{
    PendingTransactionError, Provider, WsConnect,
    network::{ReceiptResponse, TransactionBuilder},
};
use alloy_rpc_types::serde_helpers::WithOtherFields;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::{SolConstructor, sol};
use gadget_clients::tangle::client::{TangleClient as TestClient, TangleConfig};
use gadget_logging::{error, info};
use sp_core::H160;
use tangle_subxt::subxt::{
    Config,
    blocks::ExtrinsicEvents,
    client::OnlineClientT,
    tx::{TxProgress, signer::Signer},
    utils::AccountId32,
};
use tangle_subxt::tangle_testnet_runtime::api::assets::events::created::AssetId;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::types::AssetSecurityCommitment;
use tangle_subxt::tangle_testnet_runtime::api::{
    self,
    runtime_types::{
        pallet_services::module::Call,
        sp_arithmetic::per_things::Percent,
        tangle_primitives::services::types::{Asset, AssetSecurityRequirement, MembershipModel},
        tangle_testnet_runtime::RuntimeCall,
    },
    services::{
        calls::types::{
            call::{Args, Job},
            create_blueprint::Blueprint,
            register::{Preferences, RegistrationArgs},
        },
        events::{JobCalled, JobResultSubmitted, MasterBlueprintServiceManagerRevised},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("Failed to find `JobCalled` event")]
    NoJobCalled,
    #[error("Failed to get job result")]
    NoJobResult,
    #[error("Service not found")]
    ServiceNotFound,
    #[error("Created service does not match blueprint ID")]
    ServiceIdMismatch,
    #[error(transparent)]
    Rpc(#[from] alloy_transport::RpcError<alloy_transport::TransportErrorKind>),
    #[error(transparent)]
    PendingTransaction(#[from] PendingTransactionError),
    #[error(transparent)]
    Subxt(#[from] tangle_subxt::subxt::Error),
    #[error("{0}")]
    Other(String),
}

sol! {
    constructor(address payable _protocolFeesReceiver);
}

pub(crate) async fn deploy_new_mbsm_revision<T: Signer<TangleConfig>>(
    evm_rpc_endpoint: &str,
    client: &TestClient,
    account_id: &T,
    signer_evm: PrivateKeySigner,
    bytecode: &[u8],
    protocol_fees_receiver: Address,
) -> Result<MasterBlueprintServiceManagerRevised, TransactionError> {
    info!("Deploying new MBSM revision ...");

    let wallet = alloy_provider::network::EthereumWallet::from(signer_evm);
    let provider = alloy_provider::ProviderBuilder::new()
        .network::<AnyNetwork>()
        .wallet(wallet)
        .on_ws(WsConnect::new(evm_rpc_endpoint))
        .await?;

    let constructor_call = constructorCall {
        _protocolFeesReceiver: protocol_fees_receiver,
    };
    let encoded_constructor = constructor_call.abi_encode();
    info!("Encoded constructor: {encoded_constructor:?}");

    let deploy_code = [bytecode, encoded_constructor.as_ref()].concat();
    info!("Deploy code length: {:?}", deploy_code.len());

    let tx = alloy_rpc_types::TransactionRequest::default()
        .with_deploy_code(deploy_code)
        .with_gas_limit(5_000_000);
    let send_result = provider.send_transaction(WithOtherFields::new(tx)).await;
    let tx = match send_result {
        Ok(tx) => tx,
        Err(err) => {
            error!("Failed to send transaction: {err}");
            return Err(err.into());
        }
    };

    // Deploy the contract.
    let tx_result = tx.get_receipt().await;
    let receipt = match tx_result {
        Ok(receipt) => receipt,
        Err(err) => {
            error!("Failed to deploy MBSM Contract: {err}");
            return Err(err.into());
        }
    };

    // Check the receipt status.
    let mbsm_address = if receipt.status() {
        ReceiptResponse::contract_address(&receipt).unwrap()
    } else {
        error!("MBSM Contract deployment failed!");
        error!("Receipt: {receipt:#?}");
        return Err(TransactionError::Other(
            "MBSM Contract deployment failed!".into(),
        ));
    };

    info!("MBSM Contract deployed at: {mbsm_address}");
    let sudo_call = api::tx().sudo().sudo(RuntimeCall::Services(
        Call::update_master_blueprint_service_manager {
            address: mbsm_address.0.0.into(),
        },
    ));
    let res = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&sudo_call, account_id)
        .await?;
    let evts = wait_for_in_block_success(res).await?;
    let ev = evts
        .find_first::<MasterBlueprintServiceManagerRevised>()?;
    match ev {
        Some(ev) => {
            Ok(ev)
        }
        None => {
            Err(TransactionError::Other(
                "no MBSM Revised Event emitted".into(),
            ))
        }
    }
}

/// Create a new blueprint
///
/// # Errors
///
/// Returns an error if the transaction fails
pub async fn create_blueprint<T: Signer<TangleConfig>>(
    client: &TestClient,
    account_id: &T,
    blueprint: Blueprint,
) -> Result<(), TransactionError> {
    let call = api::tx().services().create_blueprint(blueprint);
    let res = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call, account_id)
        .await?;
    wait_for_in_block_success(res).await?;
    Ok(())
}

/// Become and operator
///
/// # Errors
///
/// Returns an error if the transaction fails
pub async fn join_operators<T: Signer<TangleConfig>>(
    client: &TestClient,
    account_id: &T,
) -> Result<(), TransactionError> {
    info!("Joining operators ...");
    let call_pre = api::tx()
        .multi_asset_delegation()
        .join_operators(1_000_000_000_000_000);
    let res_pre = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call_pre, account_id)
        .await?;

    wait_for_in_block_success(res_pre).await?;
    Ok(())
}

/// Register as an operator for `blueprint_id`
///
/// # Errors
///
/// Returns an error if the transaction fails
pub async fn register_for_blueprint<T: Signer<TangleConfig>>(
    client: &TestClient,
    account_id: &T,
    blueprint_id: u64,
    preferences: Preferences,
    registration_args: RegistrationArgs,
    value: u128,
) -> Result<(), TransactionError> {
    info!("Registering to blueprint {blueprint_id} to become an operator ...");
    let call = api::tx()
        .services()
        .register(blueprint_id, preferences, registration_args, value);
    let res = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call, account_id)
        .await?;
    wait_for_in_block_success(res).await?;
    Ok(())
}

/// Submit a job call for the given `service_id`
///
/// This will submit the job, and verify that there is a `JobCalled` event with the correct `service_id`,
/// `job_id`, `caller`, and in the future, `call_id`.
///
/// # Errors
///
/// No matching `JobCalled` event is found
pub async fn submit_job<T: Signer<TangleConfig>>(
    client: &TestClient,
    user: &T,
    service_id: u64,
    job_id: Job,
    job_params: Args,
    _call_id: u64, // TODO: Actually verify this
) -> Result<JobCalled, TransactionError> {
    let call = api::tx().services().call(service_id, job_id, job_params);
    let events = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call, user)
        .await?
        .wait_for_finalized_success()
        .await?;

    let job_called_events = events.find::<JobCalled>().collect::<Vec<_>>();
    for job_called in job_called_events {
        let job_called = job_called?;
        if job_called.service_id == service_id
            && job_called.job == job_id
            && user.account_id() == job_called.caller
        {
            return Ok(job_called);
        }
    }

    Err(TransactionError::NoJobCalled)
}

/// Requests a service with a given blueprint.
///
/// This is meant for testing.
///
/// `user` will be the only permitted caller, and all `test_nodes` will be selected as operators.
///
/// # Errors
///
/// Returns an error if the transaction fails
#[allow(clippy::cast_possible_truncation)]
pub async fn request_service<T: Signer<TangleConfig>>(
    client: &TestClient,
    user: &T,
    blueprint_id: u64,
    test_nodes: Vec<AccountId32>,
    value: u128,
    optional_assets: Option<Vec<AssetSecurityRequirement<AssetId>>>,
) -> Result<(), TransactionError> {
    info!(requester = ?user.account_id(), ?test_nodes, %blueprint_id, "Requesting service");
    let min_operators = test_nodes.len() as u32;
    let security_requirements = optional_assets.unwrap_or_else(|| {
        vec![AssetSecurityRequirement {
            asset: Asset::Custom(0),
            min_exposure_percent: Percent(50),
            max_exposure_percent: Percent(80),
        }]
    });
    let call = api::tx().services().request(
        None,
        blueprint_id,
        Vec::new(),
        test_nodes,
        Vec::new(),
        security_requirements,
        1000,
        Asset::Custom(0),
        value,
        MembershipModel::Fixed { min_operators },
    );
    let res = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call, user)
        .await?;
    wait_for_in_block_success(res).await?;
    Ok(())
}

async fn wait_for_in_block_success<T: Config, C: OnlineClientT<T>>(
    mut res: TxProgress<T, C>,
) -> Result<ExtrinsicEvents<T>, TransactionError> {
    let mut val = Err("Failed to get in block success".into());
    while let Some(Ok(event)) = res.next().await {
        let Some(block) = event.as_in_block() else {
            continue;
        };
        val = block.wait_for_success().await;
    }

    val.map_err(Into::into)
}

pub(crate) async fn wait_for_completion_of_tangle_job(
    client: &TestClient,
    service_id: u64,
    call_id: u64,
    required_count: usize,
) -> Result<JobResultSubmitted, TransactionError> {
    let mut count = 0;
    let mut blocks = client.subxt_client().blocks().subscribe_best().await?;
    while let Some(Ok(block)) = blocks.next().await {
        let events = block.events().await?;
        let results = events.find::<JobResultSubmitted>().collect::<Vec<_>>();
        info!(
            %service_id,
            %call_id,
            %required_count,
            %count,
            "Waiting for job completion. Found {} results ...",
            results.len()
        );
        for result in results {
            match result {
                Ok(result) => {
                    if result.service_id == service_id && result.call_id == call_id {
                        count += 1;
                        if count == required_count {
                            return Ok(result);
                        }
                    }
                }
                Err(err) => {
                    error!("Failed to get job result: {err}");
                }
            }
        }
    }

    Err(TransactionError::NoJobResult)
}

async fn get_next_service_id(client: &TestClient) -> Result<u64, TransactionError> {
    let call = api::storage().services().next_instance_id();
    let res = client
        .subxt_client()
        .storage()
        .at_latest()
        .await?
        .fetch_or_default(&call)
        .await?;
    Ok(res)
}

pub(crate) async fn get_latest_mbsm_revision(
    client: &TestClient,
) -> Result<Option<(u64, H160)>, TransactionError> {
    let call = api::storage()
        .services()
        .master_blueprint_service_manager_revisions();
    let mut res = client
        .subxt_client()
        .storage()
        .at_latest()
        .await?
        .fetch_or_default(&call)
        .await?;
    let ver = res.0.len() as u64;
    Ok(res.0.pop().map(|addr| (ver, addr.0.into())))
}

#[must_use]
pub fn get_security_requirement(a: AssetId, p: &[u8; 2]) -> AssetSecurityRequirement<AssetId> {
    AssetSecurityRequirement {
        asset: Asset::Custom(a),
        min_exposure_percent: Percent(p[0]),
        max_exposure_percent: Percent(p[1]),
    }
}

#[must_use]
pub fn get_security_commitment(a: Asset<AssetId>, p: u8) -> AssetSecurityCommitment<AssetId> {
    AssetSecurityCommitment {
        asset: a,
        exposure_percent: Percent(p),
    }
}

/// Approves a service request. This is meant for testing, and will always approve the request.
async fn approve_service<T: Signer<TangleConfig>>(
    client: &TestClient,
    caller: &T,
    request_id: u64,
    restaking_percent: u8,
    optional_assets: Option<Vec<AssetSecurityCommitment<AssetId>>>,
) -> Result<(), TransactionError> {
    info!("Approving service request ...");
    let native_security_commitments =
        vec![get_security_commitment(Asset::Custom(0), restaking_percent)];
    let security_commitments = match optional_assets {
        Some(assets) => [native_security_commitments, assets].concat(),
        None => native_security_commitments,
    };

    let call = api::tx()
        .services()
        .approve(request_id, security_commitments);
    let res = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call, caller)
        .await?;
    res.wait_for_finalized_success().await?;
    Ok(())
}

async fn get_next_request_id(client: &TestClient) -> Result<u64, TransactionError> {
    info!("Fetching next request ID ...");
    let next_request_id_addr = api::storage().services().next_service_request_id();
    let next_request_id = client
        .subxt_client()
        .storage()
        .at_latest()
        .await?
        .fetch_or_default(&next_request_id_addr)
        .await?;
    Ok(next_request_id)
}

pub(crate) async fn setup_operator_and_service_multiple<T: Signer<TangleConfig>>(
    clients: &[TestClient],
    sr25519_signers: &[T],
    blueprint_id: u64,
    preferences: &[Preferences],
    _exit_after_registration: bool,
) -> Result<u64, TransactionError> {
    let alice_signer = sr25519_signers
        .first()
        .ok_or(TransactionError::Other("No signers".to_string()))?;

    let alice_client = clients
        .first()
        .ok_or(TransactionError::Other("No client".to_string()))?;

    for ((operator, client), preferences) in sr25519_signers.iter().zip(clients).zip(preferences) {
        join_operators(client, operator).await?;
        // Register for blueprint
        register_for_blueprint(
            client,
            operator,
            blueprint_id,
            preferences.clone(),
            RegistrationArgs::new(),
            0,
        )
        .await?;
    }

    // Get the current service ID before requesting new service
    let prev_service_id = get_next_service_id(alice_client).await?;

    let accounts = sr25519_signers
        .iter()
        .map(Signer::account_id)
        .collect::<Vec<_>>();
    request_service(alice_client, alice_signer, blueprint_id, accounts, 0, None).await?;

    // Approve the service request and wait for completion
    let request_id = get_next_request_id(alice_client).await?.saturating_sub(1);

    for (signer, client) in sr25519_signers.iter().zip(clients) {
        approve_service(client, signer, request_id, 50, None).await?;
    }

    // Get the new service ID from events
    let new_service_id = get_next_service_id(alice_client).await?;
    assert!(new_service_id > prev_service_id);

    // Verify the service belongs to our blueprint
    let service = alice_client
        .subxt_client()
        .storage()
        .at_latest()
        .await?
        .fetch(
            &api::storage()
                .services()
                .instances(new_service_id.saturating_sub(1)),
        )
        .await?
        .ok_or(TransactionError::ServiceNotFound)?;

    if service.blueprint != blueprint_id {
        return Err(TransactionError::ServiceIdMismatch);
    }
    Ok(new_service_id.saturating_sub(1))
}
