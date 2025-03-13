use color_eyre::eyre::Result;
use gadget_chain_setup::tangle::deploy::{Opts as DeployOpts, deploy_to_tangle};
use gadget_crypto::sp_core::{SpEcdsa, SpSr25519};
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use gadget_logging::{info, setup_log};
use gadget_std::time::Duration;
use gadget_testing_utils::tangle::harness::generate_env_from_node_id;
use gadget_testing_utils::{harness::TestHarness, tangle::TangleTestHarness};
use tangle_subxt::subxt::tx::Signer;
use tokio::fs;

use crate::commands;
use crate::tests::tangle::blueprint::create_test_blueprint;

#[tokio::test]
async fn test_register_request_and_list() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    info!("Generating temporary Blueprint files");
    let (temp_dir, blueprint_dir) = create_test_blueprint();
    let temp_path = temp_dir.path().to_path_buf();
    let deploy_dir = temp_path.join("deploy_dir");
    fs::create_dir_all(&deploy_dir).await?;

    std::env::set_current_dir(&blueprint_dir)?;

    let harness = TangleTestHarness::setup(temp_dir, ()).await?;

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
        pkg_name: None,
        http_rpc_url: harness.http_endpoint.to_string(),
        ws_rpc_url: harness.ws_endpoint.to_string(),
        manifest_path: blueprint_dir.join("Cargo.toml"),
        signer: Some(deployment_signer.clone()),
        signer_evm: Some(deployment_alloy_key.clone()),
    };

    let blueprint_id = deploy_to_tangle(deploy_opts).await?;

    commands::register(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        env.keystore_uri.clone(),
    )
    .await?;

    let request_result = commands::request_service(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        10,
        20,
        vec![alice_account],
        0,
        deployment_env.keystore_uri.clone(),
    )
    .await;
    assert!(request_result.is_ok());

    let result = commands::list_requests(env.ws_rpc_endpoint.clone()).await;
    assert!(result.is_ok());

    let requests = result.unwrap();
    assert!(
        !requests.is_empty(),
        "Expected at least one request to be listed"
    );

    let first_request = &requests[0].1;
    assert_eq!(
        first_request.blueprint, blueprint_id,
        "Blueprint ID mismatch"
    );

    Ok(())
}

#[tokio::test]
async fn test_accept_request() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    info!("Generating temporary Blueprint files");
    let (temp_dir, blueprint_dir) = create_test_blueprint();
    let temp_path = temp_dir.path().to_path_buf();
    let deploy_dir = temp_path.join("deploy_dir");
    fs::create_dir_all(&deploy_dir).await?;

    std::env::set_current_dir(&blueprint_dir)?;

    let harness = TangleTestHarness::setup(temp_dir, ()).await?;

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
        pkg_name: None,
        http_rpc_url: harness.http_endpoint.to_string(),
        ws_rpc_url: harness.ws_endpoint.to_string(),
        manifest_path: blueprint_dir.join("Cargo.toml"),
        signer: Some(deployment_signer.clone()),
        signer_evm: Some(deployment_alloy_key.clone()),
    };

    let blueprint_id = deploy_to_tangle(deploy_opts).await?;

    commands::register(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        env.keystore_uri.clone(),
    )
    .await?;

    let request_result = commands::request_service(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        10,
        20,
        vec![alice_account],
        0,
        deployment_env.keystore_uri.clone(),
    )
    .await;
    assert!(request_result.is_ok());

    let requests = commands::list_requests(env.ws_rpc_endpoint.clone()).await?;
    assert!(
        !requests.is_empty(),
        "Expected at least one request to be created"
    );
    let request_id = requests[0].0;

    let result = commands::accept_request(
        env.ws_rpc_endpoint.clone(),
        10,
        20,
        15,
        env.keystore_uri.clone(),
        request_id,
    )
    .await;

    assert!(result.is_ok());

    tokio::time::sleep(Duration::from_secs(2)).await;

    let updated_requests = commands::list_requests(env.ws_rpc_endpoint.clone()).await?;
    let request_still_exists = updated_requests.iter().any(|(id, _)| *id == request_id);
    assert!(
        !request_still_exists,
        "Request should have been accepted and removed from the list"
    );

    Ok(())
}

#[tokio::test]
async fn test_reject_request() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    info!("Generating temporary Blueprint files");
    let (temp_dir, blueprint_dir) = create_test_blueprint();
    let temp_path = temp_dir.path().to_path_buf();
    let deploy_dir = temp_path.join("deploy_dir");
    fs::create_dir_all(&deploy_dir).await?;

    std::env::set_current_dir(&blueprint_dir)?;

    let harness = TangleTestHarness::setup(temp_dir, ()).await?;

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
        pkg_name: None,
        http_rpc_url: harness.http_endpoint.to_string(),
        ws_rpc_url: harness.ws_endpoint.to_string(),
        manifest_path: blueprint_dir.join("Cargo.toml"),
        signer: Some(deployment_signer.clone()),
        signer_evm: Some(deployment_alloy_key.clone()),
    };

    let blueprint_id = deploy_to_tangle(deploy_opts).await?;

    commands::register(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        env.keystore_uri.clone(),
    )
    .await?;

    let request_result = commands::request_service(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        10,
        20,
        vec![alice_account],
        0,
        deployment_env.keystore_uri.clone(),
    )
    .await;
    assert!(request_result.is_ok());

    let requests = commands::list_requests(env.ws_rpc_endpoint.clone()).await?;
    assert!(
        !requests.is_empty(),
        "Expected at least one request to be created"
    );
    let request_id = requests[0].0;

    let result = commands::reject_request(
        env.ws_rpc_endpoint.clone(),
        env.keystore_uri.clone(),
        request_id,
    )
    .await;

    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_submit_job() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    info!("Generating temporary Blueprint files");
    let (temp_dir, blueprint_dir) = create_test_blueprint();
    let temp_path = temp_dir.path().to_path_buf();
    let deploy_dir = temp_path.join("deploy_dir");
    fs::create_dir_all(&deploy_dir).await?;

    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(&blueprint_dir)?;

    let harness = TangleTestHarness::setup(temp_dir, ()).await?;

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
        pkg_name: None,
        http_rpc_url: harness.http_endpoint.to_string(),
        ws_rpc_url: harness.ws_endpoint.to_string(),
        manifest_path: blueprint_dir.join("Cargo.toml"),
        signer: Some(deployment_signer.clone()),
        signer_evm: Some(deployment_alloy_key.clone()),
    };

    let blueprint_id = deploy_to_tangle(deploy_opts).await?;

    commands::register(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        env.keystore_uri.clone(),
    )
    .await?;

    commands::request_service(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        10,
        20,
        vec![alice_account.clone()],
        0,
        deployment_env.keystore_uri.clone(),
    )
    .await?;

    let requests = commands::list_requests(env.ws_rpc_endpoint.clone()).await?;
    let request = requests.first().unwrap();
    let request_id = request.0;

    commands::accept_request(
        env.ws_rpc_endpoint.clone(),
        10,
        20,
        15,
        env.keystore_uri.clone(),
        request_id,
    )
    .await?;

    std::env::set_current_dir(original_dir)?;

    let job_args_file = temp_path.join("job_args.json");
    let job_args_content = r"[2]";
    fs::write(&job_args_file, job_args_content).await?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    let result = commands::submit_job(
        env.ws_rpc_endpoint.clone(),
        Some(0),
        blueprint_id,
        deployment_env.keystore_uri.clone(),
        0,
        Some(job_args_file.to_string_lossy().to_string()),
        false,
    )
    .await;

    assert!(result.is_ok());

    let job_called = result.unwrap();
    assert_eq!(job_called.service_id, 0, "Service ID mismatch");
    assert_eq!(job_called.job, 0, "Job ID mismatch");

    Ok(())
}
