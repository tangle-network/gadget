use crate::examples::eigen_context;
use crate::examples::eigen_context::ExampleTaskManager;
use alloy_provider::Provider;
use blueprint_test_utils::eigenlayer_test_env::{
    AVS_DIRECTORY_ADDR, DELEGATION_MANAGER_ADDR, OPERATOR_STATE_RETRIEVER_ADDR,
    REGISTRY_COORDINATOR_ADDR, STRATEGY_MANAGER_ADDR,
};
use blueprint_test_utils::helpers::get_receipt;
use blueprint_test_utils::incredible_squaring_helpers::start_default_anvil_testnet;
use blueprint_test_utils::{inject_test_keys, KeyGenType};
use gadget_sdk::config::{ContextConfig, GadgetCLICoreSettings, Protocol};
use gadget_sdk::info;
use gadget_sdk::logging::setup_log;
use gadget_sdk::runners::eigenlayer::EigenlayerConfig;
use gadget_sdk::runners::BlueprintRunner;
use gadget_sdk::utils::evm::get_provider_http;
use reqwest::Url;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;

#[tokio::test]
async fn test_eigenlayer_context() {
    setup_log();

    let (_container, http_endpoint, ws_endpoint) = start_default_anvil_testnet(false).await;
    let url = Url::parse(&http_endpoint).unwrap();
    let target_port = url.port().unwrap();

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

            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(format!(
                    "cast rpc anvil_mine 1 --rpc-url {} > /dev/null",
                    http_endpoint
                ))
                .output()
                .await
                .unwrap();
            info!("Mined a block...");
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

    // Create the GadgetConfiguration
    let config = ContextConfig {
        gadget_core_settings: GadgetCLICoreSettings::Run {
            target_addr: IpAddr::from_str("127.0.0.1").unwrap(),
            target_port,
            use_secure_url: false,
            test_mode: false,
            log_id: None,
            http_rpc_url: url,
            bootnodes: None,
            keystore_uri: keystore_uri_str,
            chain: gadget_io::SupportedChains::LocalTestnet,
            verbose: 3,
            pretty: true,
            keystore_password: None,
            blueprint_id: Some(0),
            service_id: Some(0),
            skip_registration: false,
            protocol: Protocol::Eigenlayer,
            registry_coordinator: Some(REGISTRY_COORDINATOR_ADDR),
            operator_state_retriever: Some(OPERATOR_STATE_RETRIEVER_ADDR),
            delegation_manager: Some(DELEGATION_MANAGER_ADDR),
            ws_rpc_url: Url::parse(&ws_endpoint).unwrap(),
            strategy_manager: Some(STRATEGY_MANAGER_ADDR),
            avs_directory: Some(AVS_DIRECTORY_ADDR),
            operator_registry: None,
            network_registry: None,
            base_delegator: None,
            network_opt_in_service: None,
            vault_opt_in_service: None,
            slasher: None,
            veto_slasher: None,
        },
    };
    let env = gadget_sdk::config::load(config).expect("Failed to load environment");

    BlueprintRunner::new(EigenlayerConfig {}, env.clone())
        .job(eigen_context::constructor(env.clone()).await.unwrap())
        .run()
        .await
        .unwrap();
}
