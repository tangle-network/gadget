use std::error::Error;
// #[cfg(test)]
// mod tests {
use super::*;
use crate::runner::EigenlayerGadgetRunner;
use alloy_network::EthereumWallet;
use alloy_primitives::address;
use alloy_provider::Provider;
use alloy_signer_local::PrivateKeySigner;
use blueprint_test_utils::test_ext::NAME_IDS;
use blueprint_test_utils::{anvil, get_receipt, inject_test_keys};
use gadget_io::SupportedChains;
use gadget_sdk::config::{ContextConfig, GadgetCLICoreSettings, Protocol};
use gadget_sdk::info;
use gadget_sdk::logging::setup_log;
use gadget_sdk::run::GadgetRunner;
use incredible_squaring_aggregator::aggregator::AggregatorContext;
use reqwest::Url;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

// alloy_sol_types::sol!(
//     #[allow(missing_docs)]
//     #[sol(rpc)]
//     IncredibleSquaringTaskManager,
//     "./contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
// );

alloy_sol_types::sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    PauserRegistry,
    "./contracts/out/IPauserRegistry.sol/IPauserRegistry.json"
);

alloy_sol_types::sol!(
    #[allow(missing_docs, clippy::too_many_arguments)]
    #[sol(rpc)]
    RegistryCoordinator,
    "./contracts/out/RegistryCoordinator.sol/RegistryCoordinator.json"
);

#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::needless_return)]
async fn test_eigenlayer_incredible_squaring_blueprint() -> Result<(), Box<dyn Error>> {
    setup_log();
    let (_container, http_endpoint, ws_endpoint) = anvil::start_anvil_container(true).await;

    let alice_keystore = setup_eigen_environment(0).await; // We use an ID of 0 for Alice

    std::env::set_var("EIGENLAYER_HTTP_ENDPOINT", http_endpoint.clone());
    std::env::set_var("EIGENLAYER_WS_ENDPOINT", ws_endpoint.clone());
    std::env::set_var("REGISTRATION_MODE_ON", "true"); // Set to run registration
    std::env::set_var("RPC_URL", "http://127.0.0.1:8545");
    std::env::set_var("KEYSTORE_URI", alice_keystore.clone());
    std::env::set_var("DATA_DIR", alice_keystore.clone());
    std::env::set_var("OPERATOR_ECDSA_KEY_PASSWORD", "ECDSA_PASSWORD");
    std::env::set_var("OPERATOR_BLS_KEY_PASSWORD", "BLS_PASSWORD");

    let http_url = Url::parse(&http_endpoint.clone()).unwrap();
    let ws_url = Url::parse(&ws_endpoint.clone()).unwrap();
    let bind_port = ws_url.port().unwrap();

    // Sleep to give the testnet time to spin up
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Create a provider using the transport
    let provider = alloy_provider::ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone()
        .boxed();
    let accounts = provider.get_accounts().await.unwrap();

    let registry_coordinator_addr = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
    let operator_state_retriever_addr = address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
    let erc20_mock_addr = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");

    // Deploy the Pauser Registry to the running Testnet
    let pauser_registry = PauserRegistry::deploy(provider.clone()).await.unwrap();
    let &pauser_registry_addr = pauser_registry.address();

    // Create Quorum
    let registry_coordinator =
        RegistryCoordinator::new(registry_coordinator_addr, provider.clone());
    let operator_set_params = RegistryCoordinator::OperatorSetParam {
        maxOperatorCount: 10,
        kickBIPsOfOperatorStake: 100,
        kickBIPsOfTotalStake: 1000,
    };
    let strategy_params = RegistryCoordinator::StrategyParams {
        strategy: erc20_mock_addr,
        multiplier: 1,
    };
    let _receipt = get_receipt(registry_coordinator.createQuorum(
        operator_set_params,
        0,
        vec![strategy_params],
    ))
    .await
    .unwrap();

    // Deploy the Incredible Squaring Task Manager to the running Testnet
    let task_manager_addr = get_receipt(IncredibleSquaringTaskManager::deploy_builder(
        provider.clone(),
        registry_coordinator_addr,
        10u32,
    ))
    .await
    .unwrap()
    .contract_address
    .unwrap();
    info!("Task Manager: {:?}", task_manager_addr);
    std::env::set_var("TASK_MANAGER_ADDRESS", task_manager_addr.to_string());

    // We create a Task Manager instance for the task spawner
    let task_manager = IncredibleSquaringTaskManager::new(task_manager_addr, provider.clone());
    let task_generator_address = accounts[4];

    // Initialize the Incredible Squaring Task Manager
    let init_receipt = get_receipt(task_manager.initialize(
        pauser_registry_addr,
        accounts[1],
        accounts[9],
        task_generator_address,
    ))
    .await
    .unwrap();
    assert!(init_receipt.status());

    // Aggregator is set as the 10th Anvil Account
    let signer: PrivateKeySigner =
        "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6"
            .parse()
            .unwrap();
    let wallet = EthereumWallet::from(signer);
    let aggregator = AggregatorContext::new(
        "127.0.0.1:8081".to_string(),
        task_manager_addr,
        http_endpoint.clone(),
        wallet,
        sdk_config,
    )
    .await?;

    // Run the server in a separate thread
    let (_aggregator_task, aggregator_shutdown) = aggregator.start(ws_endpoint.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(3000)).await;

    // Create the Runner with the necessary configs
    let config = ContextConfig {
        gadget_core_settings: GadgetCLICoreSettings::Run {
            bind_addr: IpAddr::from_str("127.0.0.1").unwrap(),
            bind_port,
            test_mode: false,
            log_id: None,
            http_rpc_url: http_url,
            ws_rpc_url: ws_url,
            bootnodes: None,
            keystore_uri: alice_keystore,
            chain: SupportedChains::LocalTestnet,
            verbose: 3,
            pretty: true,
            keystore_password: None,
            blueprint_id: 0,
            service_id: Some(0),
            protocol: Protocol::Eigenlayer,
        },
    };
    let env = gadget_sdk::config::load(config).expect("Failed to load environment");
    let mut runner = Box::new(EigenlayerGadgetRunner::new(env.clone()).await);

    info!("~~~ Executing the incredible squaring blueprint ~~~");

    info!("Registering...");
    if env.should_run_registration() {
        runner.register().await.unwrap();
    }

    // Deploy a new task
    if get_receipt(
        task_manager
            .createNewTask(U256::from(2), 100u32, Bytes::from(vec![0]))
            .from(task_generator_address),
    )
    .await
    .unwrap()
    .status()
    {
        info!("Deployed a new task");
    }

    // Start the Chain Updater Spawner
    let operators = vec![vec![accounts[0]]];
    let quorums = Bytes::from(vec![0]);
    let chain_updater = async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;

            // Update the operators for quorum 0 to keep StakeRegistry updates within withdrawalDelayBlocks window
            if get_receipt(
                registry_coordinator.updateOperatorsForQuorum(operators.clone(), quorums.clone()),
            )
            .await
            .unwrap()
            .status()
            {
                info!("Updated operators for quorum 0");
            }

            // Mine a block
            mine_blocks(&http_endpoint, 1).await;
        }
    };
    let _chain_task = tokio::spawn(chain_updater);

    info!("Running...");
    let runner_task = async move {
        runner.run().await.unwrap();
    };
    let _runner_task = tokio::spawn(runner_task);

    tokio::time::sleep(tokio::time::Duration::from_secs(12)).await;

    let latest_task_num = task_manager.latestTaskNum().call().await.unwrap()._0;

    let task_hash = task_manager
        .allTaskHashes(latest_task_num - 1)
        .call()
        .await
        .unwrap()
        ._0;
    assert_ne!(FixedBytes::<32>::default(), task_hash);

    let response_hash = task_manager
        .allTaskResponses(latest_task_num - 1)
        .call()
        .await
        .unwrap()
        ._0;
    assert_ne!(FixedBytes::<32>::default(), response_hash);

    info!("Shutting down aggregator...");
    aggregator_shutdown.send(()).unwrap();
    // chain_task.abort();
    // runner_task.abort();
    // aggregator_task.abort();
    // cancellation_token.cancel();

    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    info!("Exiting Successfully...");
    return Ok(()); // TODO: Aggregator Server tends to hang and keeps test from completing
}

async fn setup_eigen_environment(id: usize) -> String {
    // Set up the Keys required for Tangle AVS
    if id >= NAME_IDS.len() {
        panic!("Invalid ID: {id}");
    }
    let name = NAME_IDS[id];
    let tmp_store = Uuid::new_v4().to_string();
    let keystore_uri = PathBuf::from(format!(
        "./target/keystores/{}/{tmp_store}/",
        name.to_lowercase()
    ));
    assert!(
        !keystore_uri.exists(),
        "Keystore URI cannot exist: {}",
        keystore_uri.display()
    );
    let keystore_uri_normalized =
        std::path::absolute(keystore_uri.clone()).expect("Failed to resolve keystore URI");
    let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());
    inject_test_keys(&keystore_uri, id)
        .await
        .expect("Failed to inject testing keys for Tangle AVS");
    keystore_uri_str
}

async fn mine_blocks(endpoint: &str, blocks: u8) {
    tokio::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cast rpc anvil_mine {} --rpc-url {} > /dev/null",
            blocks, endpoint
        ))
        .output()
        .await
        .unwrap();
    info!(
        "Mined {} block{}",
        blocks,
        if blocks == 1 { "" } else { "s" }
    );
}
