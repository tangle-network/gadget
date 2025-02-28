use alloy_signer_local::PrivateKeySigner;
use color_eyre::eyre::{Result, eyre};
use gadget_blueprint_manager::executor::run_blueprint_manager;
use gadget_client_tangle::client::TangleClient;
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use sp_core::sr25519;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::{self, call::Job};
use tangle_subxt::tangle_testnet_runtime::api::services::events::{JobCalled, JobResultSubmitted};

#[derive(Clone)]
pub struct RunOpts {
    /// The HTTP RPC URL of the Tangle Network
    pub http_rpc_url: String,
    /// The WS RPC URL of the Tangle Network
    pub ws_rpc_url: String,
    /// The signer for Tangle operations
    pub signer: Option<TanglePairSigner<sr25519::Pair>>,
    /// The signer for EVM operations
    pub signer_evm: Option<PrivateKeySigner>,
    /// The blueprint ID to run
    pub blueprint_id: Option<u64>,
}

/// Lists all service requests for the current account
pub async fn list_service_requests(client: &TangleClient) -> Result<Vec<types::ServiceRequest>> {
    let requests = client.get_service_requests().await?;
    Ok(requests)
}

/// Responds to a service request
pub async fn respond_to_service_request(
    client: &TangleClient,
    request_id: u64,
    accept: bool,
) -> Result<()> {
    if accept {
        client.accept_service_request(request_id).await?;
    } else {
        client.reject_service_request(request_id).await?;
    }
    Ok(())
}

/// Submits a new service request
pub async fn request_service(
    client: &TangleClient,
    service_id: u64,
    preferences: types::register::Preferences,
) -> Result<u64> {
    let request_id = client.request_service(service_id, preferences).await?;
    Ok(request_id)
}

/// Submits a job to a service
pub async fn submit_job(
    client: &TangleClient,
    service_id: u64,
    job_id: u8,
    inputs: Vec<types::JobInput>,
) -> Result<JobCalled> {
    let job = Job { id: job_id, inputs };
    let submitted = client.submit_job(service_id, job).await?;
    Ok(submitted)
}

/// Waits for a job to complete and returns the result
pub async fn wait_for_job_result(
    client: &TangleClient,
    service_id: u64,
    job: JobCalled,
) -> Result<JobResultSubmitted> {
    let result = client.wait_for_job_result(service_id, job).await?;
    Ok(result)
}

/// Runs a blueprint using the blueprint manager
pub async fn run_blueprint(opts: RunOpts) -> Result<()> {
    let blueprint_id = opts.blueprint_id.ok_or_else(|| eyre!("Blueprint ID is required"))?;
    
    // Initialize the blueprint manager configuration
    let config = gadget_blueprint_manager::config::Config {
        http_rpc_url: opts.http_rpc_url,
        ws_rpc_url: opts.ws_rpc_url,
        blueprint_id,
        signer: opts.signer,
        signer_evm: opts.signer_evm,
    };

    // Run the blueprint manager
    run_blueprint_manager(config).await?;
    Ok(())
}
