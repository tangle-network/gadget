use alloy_provider::Provider;
use blueprint_test_utils::helpers::BlueprintProcessManager;
use blueprint_test_utils::incredible_squaring_helpers::{
    start_default_anvil_testnet, wait_for_responses,
};
use gadget_sdk::config::Protocol;
use gadget_sdk::logging::setup_log;
use std::path::PathBuf;

use crate::{IncredibleSquaringTaskManager, PauserRegistry, RegistryCoordinator};
use blueprint_test_utils::eigenlayer_test_env::*;

const ANVIL_STATE_PATH: &str =
    "./blueprint-test-utils/anvil/deployed_anvil_states/testnet_state.json";

#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::needless_return)]
async fn test_eigenlayer_incredible_squaring_blueprint_from_binary() {
    setup_log();

    let (_container, http_endpoint, ws_endpoint) = start_default_anvil_testnet(true).await;

    let EigenlayerTestEnvironment {
        accounts,
        http_endpoint,
        ws_endpoint,
        eigenlayer_contract_addresses:
            EigenlayerContractAddresses {
                registry_coordinator_address,
                ..
            },
        pauser_registry_address,
        ..
    } = setup_eigenlayer_test_environment(&http_endpoint, &ws_endpoint).await;
    let owner_address = &accounts[1];
    let aggregator_address = &accounts[9];
    let task_generator_address = &accounts[4];
    let task_manager_address = deploy_task_manager(
        &http_endpoint,
        registry_coordinator_address,
        pauser_registry_address,
        *owner_address,
        *aggregator_address,
        *task_generator_address,
    )
    .await;

    let num_successful_responses_required = 3;
    let successful_responses = Arc::new(Mutex::new(0));
    let successful_responses_clone = successful_responses.clone();

    // Start the Task Response Listener
    let response_listener = setup_task_response_listener(
        task_manager_address,
        ws_endpoint.clone(),
        successful_responses,
    )
    .await;

    // Start the Task Spawner
    let task_spawner = setup_task_spawner(
        task_manager_address,
        registry_coordinator_address,
        *task_generator_address,
        accounts.to_vec(),
        http_endpoint.clone(),
    )
    .await;

    tokio::spawn(async move {
        task_spawner.await;
    });

    tokio::spawn(async move {
        response_listener.await;
    });

    info!("Starting Blueprint Binary...");

    let blueprint_process_manager = BlueprintProcessManager::new();
    let current_dir = std::env::current_dir().unwrap();
    let xsquare_task_program_path = PathBuf::from(format!(
        "{}/../../target/release/incredible-squaring-blueprint-eigenlayer",
        current_dir.display()
    ))
    .canonicalize()
    .unwrap();

    let tmp_dir = tempfile::TempDir::new().unwrap();
    let keystore_path = &format!("{}", tmp_dir.path().display());

    blueprint_process_manager
        .start_blueprints(
            vec![xsquare_task_program_path],
            &http_endpoint,
            ws_endpoint.as_ref(),
            Protocol::Eigenlayer,
            keystore_path,
        )
        .await
        .unwrap();

    // Wait for the process to complete or timeout
    let timeout_duration = Duration::from_secs(300);
    let result = wait_for_responses(
        successful_responses_clone.clone(),
        num_successful_responses_required,
        timeout_duration,
    )
    .await;

    // Check the result
    if let Ok(Ok(())) = result {
        info!("Test completed successfully with {num_successful_responses_required} tasks responded to.");
        blueprint_process_manager
            .kill_all()
            .await
            .unwrap_or_else(|e| {
                error!("Failed to kill all blueprint processes: {:?}", e);
            });
    } else {
        panic!(
            "Test timed out after {} seconds with {} successful responses out of {} required",
            timeout_duration.as_secs(),
            successful_responses_clone.lock().await,
            num_successful_responses_required
        );
    }
}

/// Sets up the test environment for the EigenLayer Blueprint.
///
/// # Description
/// - Sets all the necessary environment variables for the necessary EigenLayer Contract Addresses.
/// - Returns a [`EigenlayerTestEnvironment`] struct containing the test environment state.
pub async fn setup_eigenlayer_test_environment(
    http_endpoint: &str,
    ws_endpoint: &str,
) -> EigenlayerTestEnvironment {
    let provider = get_provider_http(http_endpoint);

    let accounts = provider.get_accounts().await.unwrap();

    let registry_coordinator_address = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
    std::env::set_var(
        "REGISTRY_COORDINATOR_ADDR",
        registry_coordinator_address.to_string(),
    );
    let operator_state_retriever_address = address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
    std::env::set_var(
        "OPERATOR_STATE_RETRIEVER_ADDR",
        operator_state_retriever_address.to_string(),
    );
    let delegation_manager_address = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
    std::env::set_var(
        "DELEGATION_MANAGER_ADDR",
        delegation_manager_address.to_string(),
    );
    let strategy_manager_address = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");
    std::env::set_var(
        "STRATEGY_MANAGER_ADDR",
        strategy_manager_address.to_string(),
    );
    let erc20_mock_address = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");
    std::env::set_var("ERC20_MOCK_ADDR", erc20_mock_address.to_string());

    let pauser_registry = PauserRegistry::deploy(provider.clone()).await.unwrap();
    let pauser_registry_address = *pauser_registry.address();

    let registry_coordinator =
        RegistryCoordinator::new(registry_coordinator_address, provider.clone());

    let operator_set_params = RegistryCoordinator::OperatorSetParam {
        maxOperatorCount: 10,
        kickBIPsOfOperatorStake: 100,
        kickBIPsOfTotalStake: 1000,
    };
    let strategy_params = RegistryCoordinator::StrategyParams {
        strategy: erc20_mock_address,
        multiplier: 1,
    };

    info!("Creating Quorum");
    let _receipt = get_receipt(registry_coordinator.createQuorum(
        operator_set_params,
        0,
        vec![strategy_params],
    ))
    .await
    .unwrap();

    info!("Setup Eigenlayer test environment");

    EigenlayerTestEnvironment {
        http_endpoint: http_endpoint.to_string(),
        ws_endpoint: ws_endpoint.to_string(),
        accounts,
        eigenlayer_contract_addresses: EigenlayerContractAddresses {
            registry_coordinator_address,
            operator_state_retriever_address,
            delegation_manager_address,
            strategy_manager_address,
            avs_directory_address: Default::default(),
        },
        pauser_registry_address,
    }
}
pub async fn deploy_task_manager(
    http_endpoint: &str,
    registry_coordinator_address: Address,
    pauser_registry_address: Address,
    owner_address: Address,
    aggregator_address: Address,
    task_generator_address: Address,
) -> Address {
    let provider = get_provider_http(http_endpoint);
    let deploy_call = IncredibleSquaringTaskManager::deploy_builder(
        provider.clone(),
        registry_coordinator_address,
        10u32,
    );
    info!("Deploying Incredible Squaring Task Manager");
    let task_manager_address = match get_receipt(deploy_call).await {
        Ok(receipt) => match receipt.contract_address {
            Some(address) => address,
            None => {
                error!("Failed to get contract address from receipt");
                panic!("Failed to get contract address from receipt");
            }
        },
        Err(e) => {
            error!("Failed to get receipt: {:?}", e);
            panic!("Failed to get contract address from receipt");
        }
    };
    info!(
        "Deployed Incredible Squaring Task Manager at {}",
        task_manager_address
    );
    std::env::set_var("TASK_MANAGER_ADDRESS", task_manager_address.to_string());

    let task_manager = IncredibleSquaringTaskManager::new(task_manager_address, provider.clone());
    // Initialize the Incredible Squaring Task Manager
    info!("Initializing Incredible Squaring Task Manager");
    let init_call = task_manager.initialize(
        pauser_registry_address,
        owner_address,
        aggregator_address,
        task_generator_address,
    );
    let init_receipt = get_receipt(init_call).await.unwrap();
    assert!(init_receipt.status());
    info!("Initialized Incredible Squaring Task Manager");

    task_manager_address
}

pub async fn setup_task_spawner(
    task_manager_address: Address,
    registry_coordinator_address: Address,
    task_generator_address: Address,
    accounts: Vec<Address>,
    http_endpoint: String,
) -> impl std::future::Future<Output = ()> {
    let provider = get_provider_http(http_endpoint.as_str());
    let task_manager = IncredibleSquaringTaskManager::new(task_manager_address, provider.clone());
    let registry_coordinator =
        RegistryCoordinator::new(registry_coordinator_address, provider.clone());

    let operators = vec![vec![accounts[0]]];
    let quorums = Bytes::from(vec![0]);
    async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(10000)).await;

            info!("Creating a new task...");
            if get_receipt(
                task_manager
                    .createNewTask(U256::from(2), 100u32, quorums.clone())
                    .from(task_generator_address),
            )
            .await
            .unwrap()
            .status()
            {
                info!("Created a new task...");
            }

            if get_receipt(
                registry_coordinator.updateOperatorsForQuorum(operators.clone(), quorums.clone()),
            )
            .await
            .unwrap()
            .status()
            {
                info!("Updated operators for quorum...");
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
    }
}

pub async fn setup_task_response_listener(
    task_manager_address: Address,
    ws_endpoint: String,
    successful_responses: Arc<Mutex<usize>>,
) -> impl std::future::Future<Output = ()> {
    let task_manager = IncredibleSquaringTaskManager::new(
        task_manager_address,
        get_provider_ws(ws_endpoint.as_str()).await,
    );

    async move {
        let filter = task_manager.TaskResponded_filter().filter;
        let mut event_stream = match task_manager.provider().subscribe_logs(&filter).await {
            Ok(stream) => stream.into_stream(),
            Err(e) => {
                error!("Failed to subscribe to logs: {:?}", e);
                return;
            }
        };
        while let Some(event) = event_stream.next().await {
            let IncredibleSquaringTaskManager::TaskResponded {
                taskResponse: _, ..
            } = event
                .log_decode::<IncredibleSquaringTaskManager::TaskResponded>()
                .unwrap()
                .inner
                .data;
            let mut counter = successful_responses.lock().await;
            *counter += 1;
        }
    }
}
