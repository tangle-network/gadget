use crate::{InputValue, OutputValue};
use alloy_primitives::Address;
use alloy_provider::network::AnyNetwork;
use alloy_provider::{
    network::{ReceiptResponse, TransactionBuilder},
    PendingTransactionError, Provider, WsConnect,
};
use alloy_rpc_types::serde_helpers::WithOtherFields;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::{sol, SolConstructor};
use gadget_clients::tangle::client::{TangleClient as TestClient, TangleConfig};
use gadget_logging::{error, info};
use sp_core::H160;
use tangle_subxt::subxt::{
    blocks::ExtrinsicEvents,
    client::OnlineClientT,
    tx::{signer::Signer, TxProgress},
    utils::AccountId32,
    Config,
};
use tangle_subxt::tangle_testnet_runtime::api::{
    self,
    runtime_types::{
        pallet_services::module::Call, sp_arithmetic::per_things::Percent,
        tangle_primitives::services::Asset, tangle_testnet_runtime::RuntimeCall,
    },
    services::{
        calls::types::{
            call::{Args, Job},
            create_blueprint::Blueprint,
            register::{Preferences, RegistrationArgs},
            request::{Assets, PaymentAsset},
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

/// Deploy a new MBSM revision and returns the result.
pub async fn deploy_new_mbsm_revision<T: Signer<TangleConfig>>(
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
        .with_recommended_fillers()
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
            address: mbsm_address.0 .0.into(),
        },
    ));
    let res = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&sudo_call, account_id)
        .await?;
    let evts = wait_for_in_block_success(res).await?;
    let ev = evts
        .find_first::<MasterBlueprintServiceManagerRevised>()?
        .expect("MBSM Revised Event to be emitted");
    Ok(ev)
}

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

pub async fn register_blueprint<T: Signer<TangleConfig>>(
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

pub async fn submit_job<T: Signer<TangleConfig>>(
    client: &TestClient,
    user: &T,
    service_id: u64,
    job_id: Job,
    job_params: Args,
    call_id: u64,
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
            && job_called.call_id == call_id
        {
            return Ok(job_called);
        }
    }

    Err(TransactionError::NoJobCalled)
}

/// Requests a service with a given blueprint. This is meant for testing, and will allow any node
/// to make a call to run a service, and will have all nodes running the service.
pub async fn request_service<T: Signer<TangleConfig>>(
    client: &TestClient,
    user: &T,
    blueprint_id: u64,
    test_nodes: Vec<AccountId32>,
    value: u128,
) -> Result<(), TransactionError> {
    let call = api::tx().services().request(
        None, // TODO: Ensure this is okay for testing
        blueprint_id,
        test_nodes.clone(),
        test_nodes,
        Default::default(),
        Assets::from([0]),
        1000,
        PaymentAsset::from(Asset::Custom(0)),
        value,
    );
    let res = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call, user)
        .await?;
    wait_for_in_block_success(res).await?;
    Ok(())
}

pub async fn wait_for_in_block_success<T: Config, C: OnlineClientT<T>>(
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

pub async fn wait_for_completion_of_tangle_job(
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

pub async fn get_next_blueprint_id(client: &TestClient) -> Result<u64, TransactionError> {
    let call = api::storage().services().next_blueprint_id();
    let res = client
        .subxt_client()
        .storage()
        .at_latest()
        .await?
        .fetch_or_default(&call)
        .await?;
    Ok(res)
}

pub async fn get_next_service_id(client: &TestClient) -> Result<u64, TransactionError> {
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

pub async fn get_next_call_id(client: &TestClient) -> Result<u64, TransactionError> {
    let call = api::storage().services().next_job_call_id();
    let res = client
        .subxt_client()
        .storage()
        .at_latest()
        .await?
        .fetch_or_default(&call)
        .await?;
    Ok(res)
}

pub async fn get_latest_mbsm_revision(
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

/// Approves a service request. This is meant for testing, and will always approve the request.
pub async fn approve_service<T: Signer<TangleConfig>>(
    client: &TestClient,
    caller: &T,
    request_id: u64,
    restaking_percent: u8,
) -> Result<(), TransactionError> {
    info!("Approving service request ...");
    let call = api::tx()
        .services()
        .approve(request_id, Percent(restaking_percent));
    let res = client
        .subxt_client()
        .tx()
        .sign_and_submit_then_watch_default(&call, caller)
        .await?;
    res.wait_for_finalized_success().await?;
    Ok(())
}

pub async fn get_next_request_id(client: &TestClient) -> Result<u64, TransactionError> {
    info!("Fetching next request ID ...");
    let next_request_id_addr = api::storage().services().next_service_request_id();
    let next_request_id = client
        .subxt_client()
        .storage()
        .at_latest()
        .await
        .expect("Failed to fetch latest block")
        .fetch_or_default(&next_request_id_addr)
        .await
        .expect("Failed to fetch next request ID");
    Ok(next_request_id)
}

/// Sets up an operator for a blueprint and returns the created service ID
/// This function:
/// 1. Joins the operator set
/// 2. Registers for the blueprint
/// 3. Requests a new service
/// 4. Approves the service request
/// 5. Returns the newly created service ID from events
pub async fn setup_operator_and_service<T: Signer<TangleConfig>>(
    client: &TestClient,
    sr25519_signer: T,
    blueprint_id: u64,
    preferences: Preferences,
    exit_after_registration: bool,
) -> Result<u64, TransactionError> {
    setup_operator_and_service_multiple(
        &[client.clone()],
        &[sr25519_signer],
        blueprint_id,
        preferences,
        exit_after_registration,
    )
    .await
}

pub async fn setup_operator_and_service_multiple<T: Signer<TangleConfig>>(
    clients: &[TestClient],
    sr25519_signers: &[T],
    blueprint_id: u64,
    preferences: Preferences,
    exit_after_registration: bool,
) -> Result<u64, TransactionError> {
    let alice_client = clients
        .first()
        .ok_or(TransactionError::Other("No client".to_string()))?;

    for (operator, client) in sr25519_signers.iter().zip(clients) {
        join_operators(client, operator).await?;
        // Register for blueprint
        register_blueprint(
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

    for (signer, client) in sr25519_signers.iter().zip(clients) {
        let account_id = signer.account_id();
        request_service(client, signer, blueprint_id, vec![account_id], 0).await?;
        // Approve the service request and wait for completion
        let request_id = get_next_request_id(client).await?.saturating_sub(1);
        approve_service(client, signer, request_id, 20).await?;
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

/// Submits a job to a service and verifies the outputs match the expected values.
///
/// # Warning
/// This method should only be used in a single node context. For multi-node testing,
/// use the multi-node test harness methods instead.
///
/// # Arguments
/// * `client` - The test client to use for submitting the job
/// * `signer` - The signer to use for submitting the job
/// * `service_id` - The ID of the service to submit the job to
/// * `job` - The job to submit
/// * `inputs` - The inputs to provide to the job
/// * `expected_outputs` - The expected outputs from the job
///
/// # Returns
/// The job results if successful
pub async fn submit_and_verify_job<T: Signer<TangleConfig>>(
    client: &TestClient,
    signer: &T,
    service_id: u64,
    job: Job,
    inputs: Vec<InputValue>,
    expected_outputs: Vec<OutputValue>,
) -> Result<JobResultSubmitted, TransactionError> {
    let job = submit_job(client, signer, service_id, job, inputs, 0).await?;
    let results = wait_for_completion_of_tangle_job(client, service_id, job.call_id, 1).await?;

    assert_eq!(
        results.result.len(),
        expected_outputs.len(),
        "Number of outputs doesn't match expected"
    );

    for (result, expected) in results.result.iter().zip(expected_outputs.iter()) {
        assert_eq!(result, expected);
    }

    Ok(results)
}
