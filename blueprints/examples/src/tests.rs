use crate::eigen_context;
use crate::eigen_context::ExampleTaskManager;
use blueprint_sdk::alloy::primitives::Address;
use blueprint_sdk::alloy::providers::Provider;
use blueprint_sdk::alloy::transports::http::reqwest::Url;
use blueprint_sdk::config::protocol::EigenlayerContractAddresses;
use blueprint_sdk::config::supported_chains::SupportedChains;
use blueprint_sdk::config::ContextConfig;
use blueprint_sdk::logging::{info, setup_log};
use blueprint_sdk::runners::core::runner::BlueprintRunner;
use blueprint_sdk::runners::eigenlayer::bls::EigenlayerBLSConfig;
use blueprint_sdk::std::path::Path;
use blueprint_sdk::std::time::Duration;
use blueprint_sdk::testing::tempfile;
use blueprint_sdk::testing::utils::anvil::keys::{inject_anvil_key, ANVIL_PRIVATE_KEYS};
use blueprint_sdk::testing::utils::anvil::{get_receipt, start_default_anvil_testnet};
use blueprint_sdk::testing::utils::harness::TestHarness;
use blueprint_sdk::testing::utils::runner::TestEnv;
use blueprint_sdk::testing::utils::tangle::{OutputValue, TangleTestHarness};
use blueprint_sdk::tokio;
use blueprint_sdk::tokio::time::timeout;
use blueprint_sdk::utils::evm::get_provider_http;
use color_eyre::Result;

#[tokio::test]
async fn test_eigenlayer_context() {
    setup_log();

    let (_container, http_endpoint, ws_endpoint) = start_default_anvil_testnet(false).await;
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
    let keystore_uri = keystore_path.join(format!("keystores/{}", uuid::Uuid::new_v4()));
    inject_anvil_key(&keystore_uri, ANVIL_PRIVATE_KEYS[1]).unwrap();
    let keystore_uri_normalized =
        std::path::absolute(&keystore_uri).expect("Failed to resolve keystore URI");
    let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());

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
        EigenlayerBLSConfig::new(Address::default(), Address::default()),
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
                    tokio::time::sleep(Duration::from_millis(3000)).await;
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
async fn test_periodic_web_poller() -> Result<()> {
    setup_log();

    // Initialize test harness
    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;

    // Setup service
    let (mut test_env, service_id, _blueprint_id) = harness.setup_services(true).await?;

    // Add the web poller job
    test_env.add_job(crate::periodic_web_poller::constructor("*/5 * * * * *"));

    // Run the test environment
    let _test_handle = tokio::spawn(async move {
        test_env.run_runner().await.unwrap();
    });

    // Wait for a few seconds to allow the job to execute
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Execute job and verify result
    let results = harness
        .execute_job(service_id, 0, vec![], vec![OutputValue::Uint64(1)])
        .await?;

    assert_eq!(results.service_id, service_id);
    Ok(())
}

#[tokio::test]
async fn test_raw_tangle_events() -> Result<()> {
    setup_log();

    // Initialize test harness
    let temp_dir = tempfile::TempDir::new()?;
    let harness = TangleTestHarness::setup(temp_dir).await?;
    let env = harness.env().clone();

    // Setup service
    let (mut test_env, service_id, _blueprint_id) = harness.setup_services(true).await?;

    // Add the raw tangle events job
    test_env.add_job(crate::raw_tangle_events::constructor(env.clone()).await?);

    // Run the test environment
    let _test_handle = tokio::spawn(async move {
        test_env.run_runner().await.unwrap();
    });

    // Wait for a few seconds to allow the job to execute
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Execute job and verify result
    let results = harness
        .execute_job(service_id, 0, vec![], vec![OutputValue::Uint64(0)])
        .await?;

    assert_eq!(results.service_id, service_id);
    Ok(())
}
