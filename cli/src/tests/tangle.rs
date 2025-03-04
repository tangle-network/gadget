use crate::commands;
use crate::run::tangle::{run_blueprint, RunOpts};
use color_eyre::Result;
use gadget_config::supported_chains::SupportedChains;
use gadget_testing_utils::harness::TestHarness;
use gadget_testing_utils::tangle::TangleTestHarness;
use std::time::Instant;
use tempfile::TempDir;
use tokio::fs;
use tokio::time::Duration;
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
    let (mut test_env, _service_id, _blueprint_id) = harness.setup_services::<1>(false).await?;
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
        Some(service_id),
        blueprint_id,
        10,
        20,
        vec![],
        1000,
        env.keystore_uri.clone(),
        None,
        SupportedChains::LocalTestnet,
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
        Some(service_id),
        blueprint_id,
        env.keystore_uri.clone(),
        None,
        SupportedChains::LocalTestnet,
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
        Some(service_id),
        blueprint_id,
        10,
        20,
        vec![],
        1000,
        env.keystore_uri.clone(),
        None,
        SupportedChains::LocalTestnet,
    )
    .await;
    assert!(request_result.is_ok());

    let result = commands::accept_request(
        env.http_rpc_endpoint.clone(),
        env.ws_rpc_endpoint.clone(),
        Some(service_id),
        blueprint_id,
        10,
        20,
        15,
        env.keystore_uri.clone(),
        None,
        SupportedChains::LocalTestnet,
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
        Some(service_id),
        blueprint_id,
        10,
        20,
        vec![],
        1000,
        env.keystore_uri.clone(),
        None,
        SupportedChains::LocalTestnet,
    )
    .await;
    assert!(request_result.is_ok());

    let result = commands::reject_request(
        env.http_rpc_endpoint.clone(),
        env.ws_rpc_endpoint.clone(),
        Some(service_id),
        blueprint_id,
        env.keystore_uri.clone(),
        None,
        SupportedChains::LocalTestnet,
        1,
    )
    .await;

    assert!(result.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_run_blueprint() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path().to_path_buf();
    let blueprint_dir = temp_path.join("simple-blueprint");
    fs::create_dir_all(&blueprint_dir).await?;

    let cargo_toml = r#"[package]
name = "simple-blueprint"
version = "0.1.0"
edition = "2021"

[dependencies]
blueprint-sdk = { git = "https://github.com/tangle-network/gadget.git", default-features = false, features = ["std", "tangle", "macros"] }
tokio = { version = "1.40", features = ["full"] }
color-eyre = "0.6"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
"#;
    fs::write(blueprint_dir.join("Cargo.toml"), cargo_toml).await?;
    fs::create_dir_all(blueprint_dir.join("src")).await?;
    let success_file = temp_dir.path().join("blueprint_success");

    let harness = TangleTestHarness::setup(temp_dir).await?;

    let main_rs = r#"use blueprint_sdk::logging::info;

#[blueprint_sdk::main(env)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("~~~ Simple Tangle Blueprint Started ~~~");
    
    // Log some information about the environment
    info!("HTTP RPC Endpoint: {}", env.http_rpc_endpoint);
    info!("WS RPC Endpoint: {}", env.ws_rpc_endpoint);
    
    // Create a success file to indicate the blueprint ran successfully
    let success_file = std::path::PathBuf::from("./blueprint_success");
    std::fs::write(success_file, "success")?;
    
    info!("Blueprint execution completed successfully");
    Ok(())
}
"#;
    fs::write(blueprint_dir.join("src/main.rs"), main_rs).await?;

    let build_output = tokio::process::Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(&blueprint_dir)
        .output()
        .await?;

    assert!(
        build_output.status.success(),
        "Failed to build blueprint: {}",
        String::from_utf8_lossy(&build_output.stderr)
    );

    let (mut test_env, _service_id, blueprint_id) = harness.setup_services::<1>(false).await?;
    test_env.initialize().await?;

    let nodes = test_env.node_handles().await;
    let node = &nodes[0];
    let env = node.gadget_config().await;

    let run_opts = RunOpts {
        http_rpc_url: env.http_rpc_endpoint.clone(),
        ws_rpc_url: env.ws_rpc_endpoint.clone(),
        blueprint_id: Some(blueprint_id),
        keystore_path: Some(env.keystore_uri.clone()),
        data_dir: Some(temp_path),
        signer: None,
        signer_evm: None,
    };

    let run_task = tokio::spawn(async move { run_blueprint(run_opts).await });

    let start_time = Instant::now();
    let timeout = Duration::from_secs(60);

    loop {
        if success_file.exists() {
            break;
        }

        if start_time.elapsed() > timeout {
            panic!("Timed out waiting for blueprint to run successfully");
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    run_task.abort();

    assert!(success_file.exists(), "Blueprint did not run successfully");

    Ok(())
}
