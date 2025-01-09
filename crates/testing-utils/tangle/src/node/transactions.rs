use alloy_provider::network::{ReceiptResponse, TransactionBuilder};
use alloy_provider::{Provider, WsConnect};
use alloy_signer_local::PrivateKeySigner;
use gadget_clients::tangle::client::TangleClient as TestClient;
use gadget_clients::tangle::client::TangleConfig;
use gadget_logging::{error, info};
use sp_core::H160;
use std::error::Error;
use subxt::blocks::ExtrinsicEvents;
use subxt::client::OnlineClientT;
use subxt::tx::signer::Signer;
use subxt::tx::TxProgress;
use subxt::utils::AccountId32;
use subxt::Config;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::pallet_services::module::Call;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_arithmetic::per_things::Percent;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::Asset;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_testnet_runtime::RuntimeCall;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::call::{Args, Job};
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::create_blueprint::Blueprint;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::register::{
    Preferences, RegistrationArgs,
};
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::request::{
    Assets, PaymentAsset,
};
use tangle_subxt::tangle_testnet_runtime::api::services::events::{
    JobCalled, JobResultSubmitted, MasterBlueprintServiceManagerRevised,
};

/// Deploy a new MBSM revision and returns the result.
pub async fn deploy_new_mbsm_revision<T: Signer<TangleConfig>>(
    evm_rpc_endpoint: &str,
    client: &TestClient,
    account_id: &T,
    signer_evm: PrivateKeySigner,
    bytecode: &[u8],
) -> Result<MasterBlueprintServiceManagerRevised, Box<dyn Error>> {
    info!("Deploying new MBSM revision ...");

    let wallet = alloy_provider::network::EthereumWallet::from(signer_evm);
    let provider = alloy_provider::ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_ws(WsConnect::new(evm_rpc_endpoint))
        .await?;

    let tx = alloy_rpc_types::TransactionRequest::default().with_deploy_code(bytecode.to_vec());
    let send_result = provider.send_transaction(tx).await;
    let tx = match send_result {
        Ok(tx) => tx,
        Err(err) => {
            error!("Failed to send transaction: {err}");
            return Err("Failed to deploy MBSM Contract".into());
        }
    };
    // Deploy the contract.
    let tx_result = tx.get_receipt().await;
    let receipt = match tx_result {
        Ok(receipt) => receipt,
        Err(err) => {
            error!("Failed to deploy MBSM Contract: {err}");
            return Err("Failed to deploy MBSM Contract".into());
        }
    };
    // Check the receipt status.
    let mbsm_address = if receipt.status() {
        ReceiptResponse::contract_address(&receipt).unwrap()
    } else {
        error!("MBSM Contract deployment failed!");
        error!("Receipt: {receipt:#?}");
        return Err("MBSM Contract deployment failed!".into());
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
) -> Result<(), Box<dyn Error>> {
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
) -> Result<(), Box<dyn Error>> {
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
) -> Result<(), Box<dyn Error>> {
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
) -> Result<JobCalled, Box<dyn Error>> {
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

    Err("Failed to find JobCalled event".into())
}

/// Requests a service with a given blueprint. This is meant for testing, and will allow any node
/// to make a call to run a service, and will have all nodes running the service.
pub async fn request_service<T: Signer<TangleConfig>>(
    client: &TestClient,
    user: &T,
    blueprint_id: u64,
    test_nodes: Vec<AccountId32>,
    value: u128,
) -> Result<(), Box<dyn Error>> {
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
) -> Result<ExtrinsicEvents<T>, Box<dyn Error>> {
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
) -> Result<JobResultSubmitted, Box<dyn Error>> {
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
    Err("Failed to get job result".into())
}

pub async fn get_next_blueprint_id(client: &TestClient) -> Result<u64, Box<dyn Error>> {
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

pub async fn get_next_service_id(client: &TestClient) -> Result<u64, Box<dyn Error>> {
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

pub async fn get_next_call_id(client: &TestClient) -> Result<u64, Box<dyn Error>> {
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
) -> Result<Option<(u64, H160)>, Box<dyn Error>> {
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
) -> Result<(), Box<dyn Error>> {
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

pub async fn get_next_request_id(client: &TestClient) -> Result<u64, Box<dyn Error>> {
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
