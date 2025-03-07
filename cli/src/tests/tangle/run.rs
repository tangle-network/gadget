use std::time::Duration;

use color_eyre::eyre::Result;
use gadget_logging::setup_log;
use gadget_testing_utils::{harness::TestHarness, tangle::TangleTestHarness};
use tempfile::TempDir;
use tokio::{fs, time::Instant};

use crate::run::tangle::{run_blueprint, RunOpts};
use crate::tests::tangle::blueprint::create_test_blueprint;
use gadget_chain_setup::tangle::deploy::{deploy_to_tangle, Opts as DeployOpts};

#[tokio::test]
async fn test_run_blueprint() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    // Use the create_test_blueprint function to create our test blueprint
    let (temp_dir, blueprint_dir) = create_test_blueprint();
    let temp_path = temp_dir.path().to_path_buf();

    // Create a success file path that we'll use to verify the blueprint ran successfully
    let success_file = temp_path.join("blueprint_success");

    // Modify the main.rs to write a success file when the blueprint runs successfully
    let main_rs_path = blueprint_dir.join("src").join("main.rs");
    let mut main_rs_content = fs::read_to_string(&main_rs_path).await?;

    // Add code to write a success file before the "Exiting..." log
    main_rs_content = main_rs_content.replace(
        "info!(\"Exiting...\");",
        &format!("info!(\"Writing success file...\");\n    std::fs::write(\"{}\", \"success\")?;\n    info!(\"Exiting...\");", 
            success_file.display().to_string().replace('\\', "/"))
    );

    fs::write(&main_rs_path, main_rs_content).await?;

    // Set the current directory to the blueprint directory
    // This is important because deploy_to_tangle looks for Cargo.toml and blueprint.json
    // in the current directory
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(&blueprint_dir)?;

    // Build the blueprint
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

    // Create the test harness in the blueprint directory
    let harness = TangleTestHarness::setup(temp_dir).await?;
    let env = harness.env();

    // Instead of using setup_services, use the CLI's deploy function
    // Create deploy options
    let deploy_opts = DeployOpts {
        pkg_name: None, // Use the current directory's package
        http_rpc_url: harness.http_endpoint.to_string(),
        ws_rpc_url: harness.ws_endpoint.to_string(),
        manifest_path: blueprint_dir.join("Cargo.toml"),
        signer: Some(harness.sr25519_signer.clone()),
        signer_evm: Some(harness.alloy_key.clone()),
    };

    // Deploy the blueprint using the CLI's deploy function
    let blueprint_id = deploy_to_tangle(deploy_opts).await?;

    // test_env.initialize().await?;

    // Reset the current directory back to the original
    std::env::set_current_dir(original_dir)?;

    // let nodes = test_env.node_handles().await;
    // let node = &nodes[0];
    // let env = node.gadget_config().await;

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
