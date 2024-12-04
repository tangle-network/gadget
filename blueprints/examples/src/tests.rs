use crate::eigen_context;
use crate::eigen_context::ExampleTaskManager;
use alloy_primitives::Address;
use alloy_provider::Provider;
use blueprint_test_utils::eigenlayer_test_env::start_default_anvil_testnet;
use blueprint_test_utils::helpers::get_receipt;
use blueprint_test_utils::{inject_test_keys, KeyGenType};
use gadget_io::SupportedChains;
use gadget_sdk::config::protocol::EigenlayerContractAddresses;
use gadget_sdk::config::ContextConfig;
use gadget_sdk::info;
use gadget_sdk::logging::setup_log;
use gadget_sdk::runners::eigenlayer::EigenlayerBLSConfig;
use gadget_sdk::runners::BlueprintRunner;
use gadget_sdk::utils::evm::get_provider_http;
use reqwest::Url;
use std::path::Path;
use std::time::Duration;
use tokio::time::timeout;

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
    inject_test_keys(&keystore_uri, KeyGenType::Anvil(1))
        .await
        .expect("Failed to inject testing keys for Blueprint Examples Test");
    let keystore_uri_normalized =
        std::path::absolute(&keystore_uri).expect("Failed to resolve keystore URI");
    let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());

    let config = ContextConfig::create_eigenlayer_config(
        url,
        Url::parse(&ws_endpoint).unwrap(),
        keystore_uri_str,
        SupportedChains::LocalTestnet,
        EigenlayerContractAddresses::default(),
    );
    let env = gadget_sdk::config::load(config).expect("Failed to load environment");

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
