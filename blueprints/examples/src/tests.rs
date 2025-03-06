use crate::eigen_context;
use crate::eigen_context::ExampleTaskManager;
use blueprint_sdk::alloy::primitives::Address;
use blueprint_sdk::alloy::providers::Provider;
use blueprint_sdk::alloy::transports::http::reqwest::Url;
use blueprint_sdk::config::protocol::EigenlayerContractAddresses;
use blueprint_sdk::config::supported_chains::SupportedChains;
use blueprint_sdk::config::{ContextConfig, BlueprintEnvironment};
use blueprint_sdk::contexts::keystore::KeystoreContext;
use blueprint_sdk::contexts::tangle::TangleClientContext;
use blueprint_sdk::crypto::sp_core::SpSr25519;
use blueprint_sdk::crypto::tangle_pair_signer::TanglePairSigner;
use blueprint_sdk::keystore::backends::Backend;
use blueprint_sdk::logging::{error, info, setup_log};
use blueprint_sdk::macros::ext::blueprint_serde::BoundedVec;
use blueprint_sdk::runners::core::runner::BlueprintRunner;
use blueprint_sdk::runners::eigenlayer::bls::EigenlayerBLSConfig;
use blueprint_sdk::std::path::Path;
use blueprint_sdk::std::rand::random;
use blueprint_sdk::std::time::Duration;
use blueprint_sdk::tangle_subxt::subxt::utils::AccountId32;
use blueprint_sdk::tangle_subxt::tangle_testnet_runtime::api::tx;
use blueprint_sdk::testing::tempfile;
use blueprint_sdk::testing::utils::anvil::keys::{inject_anvil_key, ANVIL_PRIVATE_KEYS};
use blueprint_sdk::testing::utils::anvil::{get_receipt, start_default_anvil_testnet};
use blueprint_sdk::testing::utils::harness::TestHarness;
use blueprint_sdk::testing::utils::tangle::{InputValue, TangleTestHarness};
use blueprint_sdk::tokio;
use blueprint_sdk::tokio::task::JoinHandle;
use blueprint_sdk::tokio::time::timeout;
use blueprint_sdk::utils::evm::get_provider_http;
use blueprint_sdk::utils::tangle::send;
use color_eyre::Result;
use blueprint_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::FieldType;

#[tokio::test]
async fn test_eigenlayer_context() {
    setup_log();

    let (_container, http_endpoint, ws_endpoint) = start_default_anvil_testnet(true).await;
    let url = Url::parse(&http_endpoint).unwrap();

    let provider = get_provider_http(&http_endpoint);
    info!("Fetching accounts");
    let accounts = provider.get_accounts().await.unwrap();

    let owner_address = accounts[1];
    let task_generator_address = accounts[4];

    info!("Deploying Example Task Manager...");
    let context_example_task_manager = ExampleTaskManager::deploy_builder(provider.clone());
    let context_example_task_manager_address = get_receipt(context_example_task_manager)
        .await
        .unwrap()
        .contract_address
        .unwrap();
    std::env::set_var(
        "EXAMPLE_TASK_MANAGER_ADDRESS",
        context_example_task_manager_address.to_string(),
    );
    let context_example_task_manager =
        ExampleTaskManager::new(context_example_task_manager_address, provider.clone());

    info!("Starting Eigenlayer Blueprint Context Test...");

    // Set up Task Spawner
    let _task_spawner_handle = tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(10000)).await;
            match get_receipt(
                context_example_task_manager
                    .createTask(owner_address)
                    .from(task_generator_address),
            )
            .await
            {
                Ok(receipt) => {
                    info!("Created task with receipt: {:?}", receipt);
                }
                Err(e) => {
                    panic!("Failed to create task: {:?}", e);
                }
            }
        }
    });

    // Set up Temporary Testing Keystore
    let tmp_dir = tempfile::TempDir::new().unwrap();
    let keystore_path = &format!("{}", tmp_dir.path().display());
    let keystore_path = Path::new(keystore_path);
    let keystore_uri = keystore_path.join(format!("keystores/{}/", uuid::Uuid::new_v4()));
    // std::fs::create_dir_all(&keystore_uri).expect("Failed to create keystore directory");
    inject_anvil_key(&keystore_uri, ANVIL_PRIVATE_KEYS[1]).unwrap();
    let keystore_uri_normalized =
        std::path::absolute(&keystore_uri).expect("Failed to resolve keystore URI");
    // Use the direct path without the file: prefix
    let keystore_uri_str = keystore_uri_normalized.display().to_string();

    let config = ContextConfig::create_eigenlayer_config(
        url,
        Url::parse(&ws_endpoint).unwrap(),
        keystore_uri_str,
        None,
        SupportedChains::LocalTestnet,
        EigenlayerContractAddresses::default(),
    );
    let env = blueprint_sdk::config::load(config).expect("Failed to load environment");

    let mut blueprint = BlueprintRunner::new(
        EigenlayerBLSConfig::new(Address::default(), Address::default())
            .with_exit_after_register(false),
        env.clone(),
    );

    let result = timeout(Duration::from_secs(90), async {
        tokio::select! {
            _ = blueprint
            .job(eigen_context::constructor(env.clone()).await.unwrap())
            .run()
            => {
                panic!("Blueprint ended unexpectedly");
            }
            _ = tokio::task::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    let result = std::env::var("EIGEN_CONTEXT_STATUS").unwrap_or_else(|_| "false".to_string());
                    match result.as_str() {
                        "true" => {
                            break;
                        }
                        _ => {
                            info!("Waiting for Eigenlayer Context Job to Successfully Finish...");
                        }
                    }
                }
            }) => {
                info!("Eigenlayer Context Test Finished Successfully");
            }
        }
    }).await;

    match result {
        Ok(_) => info!("Success! Exiting..."),
        Err(_) => panic!("Test timed out"),
    }
}

#[tokio::test]
#[ignore] // TODO: waiting for the new event listener
async fn test_periodic_web_poller() -> Result<()> {
    setup_log();

    // Initialize test harness
    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;

    // Setup service
    let (mut test_env, service_id, _blueprint_id) = harness.setup_services::<1>(false).await?;

    // Add the web poller job
    test_env
        .add_job(|_env| async move {
            Ok::<_, ()>(crate::periodic_web_poller::constructor("*/5 * * * * *"))
        })
        .await
        .unwrap();

    // Run the test environment
    test_env.start().await?;

    // Execute job and verify result
    let result = tokio::select! {
        result = harness.submit_job(service_id, 1, vec![]) => {
            match result {
                Ok(_) => {Ok(())},
                Err(e) => Err(e),
            }
        }
        _ = tokio::task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                let result = std::env::var("WEB_POLLER_RESULT").unwrap_or_else(|_| "0".to_string());
                match result.as_str() {
                    "3" => {
                        break;
                    }
                    _ => {
                        info!("Waiting for WEB_POLLER_RESULT to be 3...");
                    }
                }
            }
        }) => {
            Ok(())
        }
    };
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
#[ignore] // TODO: waiting for the new event listener
async fn test_services_context() -> Result<()> {
    setup_log();

    // Initialize test harness
    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;

    // Setup service
    let (mut test_env, service_id, _blueprint_id) = harness.setup_services::<1>(false).await?;

    // Add the raw tangle events job
    test_env
        .add_job(|env| async move { crate::services_context::constructor(env).await })
        .await?;

    // Run the test environment
    let _test_handle = tokio::spawn(async move {
        test_env.start().await.unwrap();
    });

    // Execute job and verify result
    let results = harness
        .submit_job(service_id, 3, vec![InputValue::List(
            FieldType::Uint8,
            BoundedVec(vec![InputValue::Uint8(0)]),
        )])
        .await?;

    assert_eq!(results.service_id, service_id);
    Ok(())
}

#[tokio::test]
#[ignore] // TODO: waiting for the new event listener
async fn test_raw_tangle_events() -> Result<()> {
    setup_log();

    // Initialize test harness
    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;
    let env = harness.env().clone();

    // Setup service
    let (mut test_env, service_id, _blueprint_id) = harness.setup_services::<1>(false).await?;

    // Add the raw tangle events job
    // test_env
    //     .add_job(|env| async move { crate::raw_tangle_events::constructor(env).await })
    //     .await?;

    // Spawn the balance transfer task
    let _handle = balance_transfer_event(env.clone()).await?;

    // Run the test environment
    test_env.start().await?;

    // Execute job and verify result
    let result = tokio::select! {
        result = harness.submit_job(service_id, 2, vec![]) => {
            match result {
                Ok(_) => {Ok(())},
                Err(e) => Err(e),
            }
        }
        _ = tokio::task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                let result = std::env::var("RAW_EVENT_RESULT").unwrap_or_else(|_| "0".to_string());
                match result.as_str() {
                    "1" => {
                        break;
                    }
                    _ => {
                        info!("Waiting for RAW_EVENT_RESULT to be 1...");
                    }
                }
            }
        }) => {
            Ok(())
        }
    };
    assert!(result.is_ok());

    Ok(())
}

async fn balance_transfer_event(env: BlueprintEnvironment) -> Result<JoinHandle<()>> {
    let client = env.tangle_client().await?;
    let transfer_client = client.clone();
    let signer = env.keystore().first_local::<SpSr25519>().unwrap();
    let sr_pair = TanglePairSigner::new(env.keystore().get_secret::<SpSr25519>(&signer).unwrap().0);
    let transfer_id = AccountId32::from(signer.0.0);
    let receiver_id = AccountId32::from(random::<[u8; 32]>());

    // Spawn task to transfer balance into Operator's account on Tangle
    let transfer_task = async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        info!(
            "Transferring balance from {:?} to {:?}",
            transfer_id, receiver_id
        );
        let transfer_tx = tx()
            .balances()
            .transfer_allow_death(receiver_id.into(), 1_000_000_000_000_000_000);
        match send(&transfer_client, &sr_pair, &transfer_tx).await {
            Ok(result) => {
                info!("Transfer Result: {:?}", result);
            }
            Err(e) => {
                error!("Balance Transfer Error: {:?}", e);
            }
        }
    };
    let transfer_handle = tokio::task::spawn(transfer_task);
    Ok(transfer_handle)
}
