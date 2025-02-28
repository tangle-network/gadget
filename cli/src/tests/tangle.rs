use crate::run::tangle::{
    list_service_requests, request_service, respond_to_service_request, run_blueprint, submit_job,
    wait_for_job_result, RunOpts,
};
use color_eyre::eyre::Result;
use gadget_client_tangle::client::TangleClient;
use gadget_testing_utils::tangle::{
    harness::TangleTestHarness,
    node::{run, NodeConfig},
};
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types;
use tempfile::TempDir;

async fn setup_test_harness() -> Result<TangleTestHarness> {
    let test_dir = TempDir::new()?;
    let harness = TangleTestHarness::setup(test_dir).await?;
    Ok(harness)
}

#[tokio::test]
async fn test_list_service_requests() -> Result<()> {
    let harness = setup_test_harness().await?;
    let client = harness.client();

    let requests = list_service_requests(client).await?;
    assert!(
        requests.is_empty(),
        "Expected no service requests initially"
    );

    let blueprint_id = harness.deploy_blueprint().await?;
    let (_test_env, service_id, _) = harness.setup_services(false).await?;

    let preferences = types::register::Preferences::default();
    let request_id = request_service(client, service_id, preferences).await?;

    let requests = list_service_requests(client).await?;
    assert_eq!(requests.len(), 1, "Expected one service request");
    assert_eq!(requests[0].id, request_id, "Request ID mismatch");
    assert_eq!(requests[0].service_id, service_id, "Service ID mismatch");

    Ok(())
}

#[tokio::test]
async fn test_respond_to_service_request() -> Result<()> {
    let harness = setup_test_harness().await?;
    let client = harness.client();

    let blueprint_id = harness.deploy_blueprint().await?;
    let (_test_env, service_id, _) = harness.setup_services(false).await?;

    let preferences = types::register::Preferences::default();
    let request_id = request_service(client, service_id, preferences).await?;

    respond_to_service_request(client, request_id, true).await?;

    let requests = list_service_requests(client).await?;
    assert_eq!(requests.len(), 1, "Expected one service request");
    assert_eq!(requests[0].id, request_id, "Request ID mismatch");
    assert!(matches!(
        requests[0].status,
        types::ServiceRequestStatus::Accepted
    ));

    Ok(())
}

#[tokio::test]
async fn test_request_service() -> Result<()> {
    let harness = setup_test_harness().await?;
    let client = harness.client();

    let blueprint_id = harness.deploy_blueprint().await?;
    let (_test_env, service_id, _) = harness.setup_services(false).await?;

    let preferences = types::register::Preferences::default();
    let request_id = request_service(client, service_id, preferences).await?;

    let requests = list_service_requests(client).await?;
    assert_eq!(requests.len(), 1, "Expected one service request");
    assert_eq!(requests[0].id, request_id, "Request ID mismatch");
    assert_eq!(requests[0].service_id, service_id, "Service ID mismatch");

    Ok(())
}

#[tokio::test]
async fn test_submit_job_and_wait_for_result() -> Result<()> {
    let harness = setup_test_harness().await?;
    let client = harness.client();

    let blueprint_id = harness.deploy_blueprint().await?;
    let (_test_env, service_id, _) = harness.setup_services(false).await?;

    let preferences = types::register::Preferences::default();
    let request_id = request_service(client, service_id, preferences).await?;
    respond_to_service_request(client, request_id, true).await?;

    let job_id = 0;
    let inputs = vec![types::JobInput::U64(42)];
    let job = submit_job(client, service_id, job_id, inputs.clone()).await?;

    let result = wait_for_job_result(client, service_id, job).await?;

    assert!(result.outputs.len() > 0, "Expected job outputs");

    Ok(())
}

#[tokio::test]
async fn test_run_blueprint() -> Result<()> {
    let harness = setup_test_harness().await?;

    let blueprint_id = harness.deploy_blueprint().await?;

    let opts = RunOpts {
        http_rpc_url: harness.http_endpoint.to_string(),
        ws_rpc_url: harness.ws_endpoint.to_string(),
        signer: Some(harness.sr25519_signer.clone()),
        signer_evm: None,
        blueprint_id: Some(blueprint_id),
    };

    run_blueprint(opts).await?;

    Ok(())
}
