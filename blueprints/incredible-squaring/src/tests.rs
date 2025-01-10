use tnt_core_bytecode::bytecode::MASTER_BLUEPRINT_SERVICE_MANAGER;
use tracing::log::debug;
use crate as blueprint;
use crate::MyContext;
use gadget_config::supported_chains::SupportedChains;
use gadget_config::ContextConfig;
use gadget_runners::tangle::{tangle::{TangleConfig}, error::TangleError};
use gadget_testing_utils::runner::TestEnv;
use gadget_testing_utils::tangle::keys::inject_tangle_key;
use gadget_testing_utils::tangle::node::{transactions, NodeConfig};
use gadget_testing_utils::tangle::runner::TangleTestEnv;
use url::Url;
use cargo_tangle::deploy::tangle::Opts;
use gadget_crypto_tangle_pair_signer::sp_core::Pair;
use gadget_macros::ext::contexts::keystore::KeystoreContext;
use gadget_macros::ext::contexts::tangle::TangleClientContext;
use gadget_crypto_tangle_pair_signer::TanglePairSigner;
use gadget_logging::setup_log;
use gadget_macros::ext::keystore::backends::tangle::TangleBackend;
use gadget_macros::ext::tangle::tangle_subxt::subxt::config::Header;
use gadget_macros::ext::tangle::tangle_subxt::subxt::tx::Signer;
use gadget_macros::ext::tangle::tangle_subxt::subxt_core::utils::AccountId32;
use gadget_macros::ext::tangle::tangle_subxt::tangle_testnet_runtime::api::services::calls::types::call::Job;
use gadget_macros::ext::tangle::tangle_subxt::tangle_testnet_runtime::api::services::calls::types::register::{Preferences, RegistrationArgs};
use gadget_macros::ext::tangle::tangle_subxt::tangle_testnet_runtime::api::services::calls::types::update_price_targets::PriceTargets;
use gadget_testing_utils::tangle::node::transactions::wait_for_completion_of_tangle_job;

pub type InputValue = gadget_macros::ext::tangle::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field<AccountId32>;
pub type OutputValue = gadget_macros::ext::tangle::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field<AccountId32>;

#[tokio::test]
async fn test_incredible_squaring() -> color_eyre::Result<()> {
    setup_log();

    // Start Local Tangle Node
    let node_config = NodeConfig::new(false);
    let tangle_node = gadget_testing_utils::tangle::node::run(node_config)
        .await
        .unwrap();

    let http_endpoint = Url::parse(&format!("http://127.0.0.1:{}", tangle_node.ws_port())).unwrap();
    let ws_endpoint = Url::parse(&format!("ws://127.0.0.1:{}", tangle_node.ws_port())).unwrap();

    // Setup testing directory
    let tmp_dir = tempfile::TempDir::new().map_err(TangleError::Io)?;
    let tmp_dir_path = tmp_dir.path().to_string_lossy().into_owned();
    inject_tangle_key(&tmp_dir_path, "//Alice")
        .map_err(|e| TangleError::Keystore(e.to_string()))?;

    let context_config = ContextConfig::create_tangle_config(
        http_endpoint,
        ws_endpoint.clone(),
        tmp_dir_path,
        None,
        SupportedChains::LocalTestnet,
        0,
        Some(0),
    );
    let env = ::gadget_macros::ext::config::load(context_config.clone())
        .expect("Failed to load environment");

    let context = MyContext {
        env: env.clone(),
        call_id: None,
    };

    let x_square = blueprint::XsquareEventHandler::new(&env, context)
        .await
        .unwrap();

    // let client = get_client(ws_endpoint.as_str(), http_endpoint.as_str()).await.unwrap();
    let client = env.tangle_client().await.unwrap();

    let sr25519_public = env.keystore().iter_sr25519().next().unwrap();
    let sr25519_pair = env
        .keystore()
        .expose_sr25519_secret(&sr25519_public)
        .unwrap()
        .unwrap();
    let sr25519_id = TanglePairSigner::new(sr25519_pair.clone());
    let account_id = sr25519_id.account_id();

    let ecdsa_public = env.keystore().iter_ecdsa().next().unwrap();
    let ecdsa_pair = env
        .keystore()
        .expose_ecdsa_secret(&ecdsa_public)
        .unwrap()
        .unwrap();
    let ecdsa_id = TanglePairSigner::new(ecdsa_pair);
    let alloy_key = ecdsa_id.alloy_key().unwrap();

    let base_path = gadget_std::env::current_dir().unwrap();
    let base_path = base_path
        .canonicalize()
        .expect("File could not be normalized");
    let manifest_path = base_path.join("Cargo.toml");
    let manifest = gadget_testing_utils::read_cargo_toml_file(&manifest_path)
        .expect("Failed to read blueprint's Cargo.toml");
    let blueprint_name = manifest.package.as_ref().unwrap().name.clone();

    let opts = Opts {
        pkg_name: Some(blueprint_name),
        http_rpc_url: format!("http://127.0.0.1:{}", tangle_node.ws_port()),
        ws_rpc_url: format!("ws://127.0.0.1:{}", tangle_node.ws_port()),
        manifest_path,
        signer: Some(sr25519_id.clone()),
        signer_evm: Some(alloy_key.clone()),
    };

    // Check if the MBSM is already deployed.
    let latest_revision = transactions::get_latest_mbsm_revision(&client)
        .await
        .expect("Get latest MBSM revision");
    match latest_revision {
        Some((rev, addr)) => debug!("MBSM is deployed at revision #{rev} at address {addr}"),
        None => {
            let bytecode = MASTER_BLUEPRINT_SERVICE_MANAGER;
            gadget_logging::trace!("Using MBSM bytecode of length: {}", bytecode.len());

            let ev = transactions::deploy_new_mbsm_revision(
                ws_endpoint.as_str(),
                &client,
                &sr25519_id,
                alloy_key.clone(),
                bytecode,
            )
            .await
            .expect("deploy new MBSM revision");
            let rev = ev.revision;
            let addr = ev.address;
            debug!("Deployed MBSM at revision #{rev} at address {addr}");
        }
    };

    // Step 1: Create the blueprint using alice's identity
    let blueprint_id = match cargo_tangle::deploy::tangle::deploy_to_tangle(opts.clone()).await {
        Ok(id) => id,
        Err(err) => {
            gadget_logging::error!("Failed to deploy blueprint: {err}");
            return Err(err);
        }
    };

    // Step 2: Have each identity register to a blueprint
    let registration_args = RegistrationArgs::new();

    let client = client.clone();
    let registration_args = registration_args.clone();

    let key =
        gadget_runners::tangle::tangle::decompress_pubkey(&ecdsa_id.signer().public().0).unwrap();

    let preferences = Preferences {
        key,
        price_targets: PriceTargets {
            cpu: 0,
            mem: 0,
            storage_hdd: 0,
            storage_ssd: 0,
            storage_nvme: 0,
        },
    };

    if let Err(err) = transactions::join_operators(&client, &sr25519_id.clone()).await {
        let err_str = format!("{err}");
        if err_str.contains("MultiAssetDelegation::AlreadyOperator") {
            gadget_logging::warn!("{} is already an operator", account_id);
        } else {
            gadget_logging::error!("Failed to join delegators: {err}");
            panic!("Failed to join delegators: {err}");
        }
    }

    if let Err(err) = transactions::register_blueprint(
        &client,
        &sr25519_id.clone(),
        blueprint_id,
        preferences,
        registration_args.clone(),
        0,
    )
    .await
    {
        gadget_logging::error!("Failed to register as operator: {err}");
        panic!("Failed to register as operator: {err}");
    }

    if let Err(err) = transactions::request_service(
        &client,
        &sr25519_id,
        blueprint_id,
        vec![account_id.clone()],
        0,
    )
    .await
    {
        gadget_logging::error!("Failed to register service: {err}");
        panic!("Failed to register service: {err}");
    }

    let next_request_id = transactions::get_next_request_id(&client)
        .await
        .expect("Failed to get next request ID")
        .saturating_sub(1);

    if let Err(err) = transactions::approve_service(&client, &sr25519_id, next_request_id, 20).await
    {
        gadget_logging::error!("Failed to approve service request {next_request_id}: {err}");
        panic!("Failed to approve service request {next_request_id}: {err}");
    }

    let now = client
        .blocks()
        .at_latest()
        .await
        .expect("Unable to get block")
        .header()
        .hash()
        .0;
    let blueprints = client
        .query_operator_blueprints(now, account_id)
        .await
        .expect("Failed to query operator blueprints");
    assert!(!blueprints.is_empty(), "No blueprints found");

    let blueprint = blueprints
        .into_iter()
        .find(|r| r.blueprint_id == blueprint_id)
        .expect("Blueprint not found in operator's blueprints");

    // Spawn job submitter
    let selected_service = &blueprint.services[0];
    let service_id = selected_service.id;
    gadget_logging::info!("Submitting job 0 with service ID {service_id}");
    let client_clone = client.clone();
    let sr25519_id_clone = sr25519_id.clone();
    tokio::spawn(async move {
        let mut test_env =
            TangleTestEnv::new(TangleConfig::default(), env, vec![x_square]).unwrap();

        test_env.run_runner().await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let job_args = vec![(InputValue::Uint64(5))];
    let job = gadget_testing_utils::tangle::node::transactions::submit_job(
        &client_clone,
        &sr25519_id_clone,
        service_id,
        Job::from(0u8),
        job_args,
        0,
    )
    .await
    .expect("Failed to submit job");

    let call_id = job.call_id;

    gadget_logging::info!("Submitted job 0 with service ID {service_id} has call id {call_id}");

    let job_results = wait_for_completion_of_tangle_job(&client, service_id, call_id, 1)
        .await
        .expect("Failed to wait for job completion");

    assert_eq!(job_results.service_id, service_id);
    assert_eq!(job_results.call_id, call_id);

    let expected_outputs = vec![OutputValue::Uint64(25)];
    if expected_outputs.is_empty() {
        gadget_logging::info!("No expected outputs specified, skipping verification");
    }

    assert_eq!(
        job_results.result.len(),
        expected_outputs.len(),
        "Number of outputs doesn't match expected"
    );

    for (result, expected) in job_results
        .result
        .into_iter()
        .zip(expected_outputs.into_iter())
    {
        assert_eq!(result, expected);
    }

    Ok(())
}
