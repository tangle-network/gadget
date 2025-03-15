use std::time::Duration;

use blueprint_core::info;
use color_eyre::eyre::Result;
use gadget_chain_setup::tangle::OutputValue;
use gadget_crypto::sp_core::{SpEcdsa, SpSr25519};
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use gadget_testing_utils::setup_log;
use gadget_testing_utils::tangle::TangleTestHarness;
use gadget_testing_utils::tangle::harness::generate_env_from_node_id;
use tangle_subxt::subxt::tx::Signer;
use tokio::fs;

use crate::command::jobs::submit::submit_job;
use crate::command::list::requests::list_requests;
use crate::command::register::register;
use crate::command::run::tangle::{RunOpts, run_blueprint};
use crate::command::service::accept::accept_request;
use crate::command::service::request::request_service;
use crate::tests::tangle::blueprint::create_test_blueprint;
use gadget_chain_setup::tangle::deploy::{Opts as DeployOpts, deploy_to_tangle};

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_run_blueprint() -> Result<()> {
    color_eyre::install()?;
    setup_log();

    info!("Generating temporary Blueprint files");
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
        pkg_name: None,
        http_rpc_url: harness.http_endpoint.to_string(),
        ws_rpc_url: harness.ws_endpoint.to_string(),
        manifest_path: blueprint_dir.join("Cargo.toml"),
        signer: Some(deployment_signer.clone()),
        signer_evm: Some(deployment_alloy_key.clone()),
    };

    let blueprint_id = deploy_to_tangle(deploy_opts).await?;

    register(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        env.keystore_uri.clone(),
    )
    .await?;

    request_service(
        env.ws_rpc_endpoint.clone(),
        blueprint_id,
        10,
        20,
        vec![alice_account.clone()],
        0,
        deployment_env.keystore_uri.clone(),
    )
    .await?;

    let requests = list_requests(env.ws_rpc_endpoint.clone()).await?;
    let request = requests.first().unwrap();
    let request_id = request.0;
    let blueprint_id = request.1.blueprint;

    accept_request(
        env.ws_rpc_endpoint.clone(),
        10,
        20,
        15,
        env.keystore_uri.clone(),
        request_id,
    )
    .await?;

    std::env::set_current_dir(original_dir)?;

    let run_opts = RunOpts {
        http_rpc_url: env.http_rpc_endpoint.clone(),
        ws_rpc_url: env.ws_rpc_endpoint.clone(),
        blueprint_id: Some(blueprint_id),
        keystore_path: Some(env.keystore_uri.clone()),
        data_dir: Some(temp_path.clone()),
        signer: Some(harness.sr25519_signer.clone()),
        signer_evm: Some(harness.alloy_key.clone()),
    };

    let _run_task = tokio::spawn(async move { run_blueprint(run_opts).await });

    info!("Running blueprint, now submitting job");

    let job_args_file = temp_path.join("job_args.json");
    let job_args_content = r"[2]"; // JSON array with a single number 2 that will be parsed as Uint64
    fs::write(&job_args_file, job_args_content).await?;
    info!("Created job arguments file at: {}", job_args_file.display());

    // We wait for the Binary to start up, otherwise it won't see the job
    // TODO: This is a hack, we should have a way to wait for the Binary to start up
    tokio::time::sleep(Duration::from_secs(80)).await;

    let job_called = submit_job(
        env.ws_rpc_endpoint.clone(),
        Some(0),
        blueprint_id,
        deployment_env.keystore_uri.clone(),
        0,
        Some(job_args_file.to_string_lossy().to_string()),
        false,
    )
    .await?;

    info!("Submitted job, now waiting for response");

    let result = harness.wait_for_job_execution(0, job_called).await.unwrap();

    harness.verify_job(&result, vec![OutputValue::Uint64(4)]);

    Ok(())
}
