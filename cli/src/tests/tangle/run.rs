use std::time::Duration;

use color_eyre::eyre::Result;
use gadget_chain_setup::tangle::transactions::{get_next_service_id, join_operators};
use gadget_chain_setup::tangle::InputValue;
use gadget_crypto::sp_core::{SpEcdsa, SpSr25519};
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use gadget_logging::setup_log;
use gadget_testing_utils::tangle::harness::generate_env_from_node_id;
use gadget_testing_utils::{harness::TestHarness, tangle::TangleTestHarness};
use tangle_subxt::subxt::tx::Signer;
use tempfile::TempDir;
use tokio::{fs, time::Instant};

use crate::run::tangle::{run_blueprint, RunOpts};
use crate::tests::tangle::blueprint::create_test_blueprint;
use gadget_chain_setup::tangle::deploy::{deploy_to_tangle, Opts as DeployOpts};

#[tokio::test]
async fn test_run_blueprint() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    let (temp_dir, blueprint_dir) = create_test_blueprint();
    let temp_path = temp_dir.path().to_path_buf();
    let deploy_dir = temp_path.join("deploy_dir");
    fs::create_dir_all(&deploy_dir).await?;

    let success_file = temp_path.join("blueprint_success");

    let main_rs_path = blueprint_dir.join("src").join("main.rs");
    let mut main_rs_content = fs::read_to_string(&main_rs_path).await?;

    // Add code to write a success file before the "Exiting..." log
    main_rs_content = main_rs_content.replace(
        "info!(\"Exiting...\");",
        &format!("info!(\"Writing success file...\");\n    std::fs::write(\"{}\", \"success\")?;\n    info!(\"Exiting...\");", 
            success_file.display().to_string().replace('\\', "/"))
    );

    fs::write(&main_rs_path, main_rs_content).await?;

    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(&blueprint_dir)?;

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

    let harness = TangleTestHarness::setup(temp_dir).await?;
    let deployment_env = generate_env_from_node_id(
        "Bob",
        harness.http_endpoint.clone(),
        harness.ws_endpoint.clone(),
        deploy_dir.as_path(),
    )
    .await?;
    let deployment_config = KeystoreConfig::new().fs_root(deployment_env.keystore_uri.clone());
    let deployment_keystore = Keystore::new(deployment_config)?;
    let deployment_sr25519 = deployment_keystore.first_local::<SpSr25519>()?;
    let deployment_pair = deployment_keystore
        .get_secret::<SpSr25519>(&deployment_sr25519)
        .unwrap();
    let deployment_signer = TanglePairSigner::new(deployment_pair.0);
    let deployment_ecdsa_public = deployment_keystore.first_local::<SpEcdsa>()?;
    let deployment_ecdsa_pair =
        deployment_keystore.get_secret::<SpEcdsa>(&deployment_ecdsa_public)?;
    let deployment_ecdsa_signer = TanglePairSigner::new(deployment_ecdsa_pair.0);
    let deployment_alloy_key = deployment_ecdsa_signer.alloy_key().unwrap();

    let env = harness.env();

    let alice_account = harness.sr25519_signer.account_id();

    let deploy_opts = DeployOpts {
        pkg_name: None, // Use the current directory's package
        http_rpc_url: harness.http_endpoint.to_string(),
        ws_rpc_url: harness.ws_endpoint.to_string(),
        manifest_path: blueprint_dir.join("Cargo.toml"),
        signer: Some(deployment_signer.clone()),
        signer_evm: Some(deployment_alloy_key.clone()),
    };

    let blueprint_id = deploy_to_tangle(deploy_opts).await?;

    // join_operators(harness.client(), &harness.sr25519_signer).await.unwrap();

    crate::commands::register(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        env.keystore_uri.clone(),
    )
    .await?;

    crate::commands::request_service(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        10,
        20,
        vec![alice_account.clone()],
        0,
        deployment_env.keystore_uri.clone(),
    )
    .await?;

    crate::commands::list_requests(env.ws_rpc_endpoint.clone()).await?;

    crate::commands::accept_request(
        env.ws_rpc_endpoint.clone(),
        10,
        20,
        15,
        env.keystore_uri.clone(),
        0,
    )
    .await?;

    // crate::commands::accept_request(ws_rpc_url, min_exposure_percent, max_exposure_percent, restaking_percent, keystore_uri, request_id)

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
        signer: Some(harness.sr25519_signer.clone()),
        signer_evm: Some(harness.alloy_key.clone()),
    };

    let run_task = tokio::spawn(async move { run_blueprint(run_opts).await });

    crate::commands::submit_job(
        env.ws_rpc_endpoint.clone(),
        Some(0),
        blueprint_id,
        env.keystore_uri.clone(),
        0,
        None,
        // vec![InputValue::Uint64(5)] // Adding a sample job argument
    )
    .await?;

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
