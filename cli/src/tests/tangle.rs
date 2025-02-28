use crate::commands;
use color_eyre::Result;
use gadget_test_harness::tangle::TangleTestHarness;
use tracing_subscriber::EnvFilter;

fn setup_log() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();
}

#[tokio::test]
async fn test_list_requests() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;
    let (mut test_env, service_id, blueprint_id) = harness.setup_services::<1>(false).await?;
    test_env.initialize().await?;

    let nodes = test_env.node_handles().await;
    let alice_node = &nodes[0];
    let env = alice_node.gadget_config().await;

    let result =
        commands::list_requests(env.http_rpc_endpoint.clone(), env.ws_rpc_endpoint.clone()).await;

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_request_service() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;
    let (mut test_env, service_id, blueprint_id) = harness.setup_services::<1>(false).await?;
    test_env.initialize().await?;

    let nodes = test_env.node_handles().await;
    let alice_node = &nodes[0];
    let env = alice_node.gadget_config().await;

    let result = commands::request_service(
        env.http_rpc_endpoint.clone(),
        env.ws_rpc_endpoint.clone(),
        service_id,
        blueprint_id,
        10,
        20,
        vec![],
        1000,
        env.keystore_uri.clone(),
        None,
        env.chain.clone(),
    )
    .await;

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_submit_job() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;
    let (mut test_env, service_id, blueprint_id) = harness.setup_services::<1>(false).await?;
    test_env.initialize().await?;

    let nodes = test_env.node_handles().await;
    let alice_node = &nodes[0];
    let env = alice_node.gadget_config().await;

    let result = commands::submit_job(
        env.http_rpc_endpoint.clone(),
        env.ws_rpc_endpoint.clone(),
        service_id,
        blueprint_id,
        env.keystore_uri.clone(),
        None,
        env.chain.clone(),
        1,
    )
    .await;

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_accept_request() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;
    let (mut test_env, service_id, blueprint_id) = harness.setup_services::<1>(false).await?;
    test_env.initialize().await?;

    let nodes = test_env.node_handles().await;
    let alice_node = &nodes[0];
    let env = alice_node.gadget_config().await;

    let request_result = commands::request_service(
        env.http_rpc_endpoint.clone(),
        env.ws_rpc_endpoint.clone(),
        service_id,
        blueprint_id,
        10,
        20,
        vec![],
        1000,
        env.keystore_uri.clone(),
        None,
        env.chain.clone(),
    )
    .await;
    assert!(request_result.is_ok());

    let result = commands::accept_request(
        env.http_rpc_endpoint.clone(),
        env.ws_rpc_endpoint.clone(),
        service_id,
        blueprint_id,
        10,
        20,
        15,
        env.keystore_uri.clone(),
        None,
        env.chain.clone(),
        1,
    )
    .await;

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_reject_request() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;
    let (mut test_env, service_id, blueprint_id) = harness.setup_services::<1>(false).await?;
    test_env.initialize().await?;

    let nodes = test_env.node_handles().await;
    let alice_node = &nodes[0];
    let env = alice_node.gadget_config().await;

    let request_result = commands::request_service(
        env.http_rpc_endpoint.clone(),
        env.ws_rpc_endpoint.clone(),
        service_id,
        blueprint_id,
        10,
        20,
        vec![],
        1000,
        env.keystore_uri.clone(),
        None,
        env.chain.clone(),
    )
    .await;
    assert!(request_result.is_ok());

    let result = commands::reject_request(
        env.http_rpc_endpoint.clone(),
        env.ws_rpc_endpoint.clone(),
        service_id,
        blueprint_id,
        env.keystore_uri.clone(),
        None,
        env.chain.clone(),
        1,
    )
    .await;

    assert!(result.is_ok());
    Ok(())
}
