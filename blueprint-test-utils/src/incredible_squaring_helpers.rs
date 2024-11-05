#[macro_export]
macro_rules! define_eigenlayer_test_env {
    () => {
        use gadget_sdk::futures::StreamExt;
        use gadget_sdk::config::protocol::EigenlayerContractAddresses;
        use gadget_sdk::utils::evm::{get_provider_http, get_provider_ws};
        use gadget_sdk::{error, info};
        use std::sync::Arc;
        use std::time::Duration;
        use gadget_sdk::tokio::sync::Mutex;
        use alloy_primitives::{Address, address, Bytes, U256};
        use alloy_provider::Provider;

        use $crate::helpers::get_receipt;

        alloy_sol_types::sol!(
            #[allow(missing_docs)]
            #[sol(rpc)]
            #[derive(Debug)]
            IncredibleSquaringTaskManager,
            "./contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
        );

        alloy_sol_types::sol!(
            #[allow(missing_docs)]
            #[sol(rpc)]
            #[derive(Debug)]
            PauserRegistry,
            "./contracts/out/IPauserRegistry.sol/IPauserRegistry.json"
        );

        alloy_sol_types::sol!(
            #[allow(missing_docs, clippy::too_many_arguments)]
            #[sol(rpc)]
            #[derive(Debug)]
            RegistryCoordinator,
            "./contracts/out/RegistryCoordinator.sol/RegistryCoordinator.json"
        );

        pub struct EigenlayerTestEnvironment {
            pub http_endpoint: String,
            pub ws_endpoint: String,
            pub accounts: Vec<Address>,
            pub eigenlayer_contract_addresses: EigenlayerContractAddresses,
            pub pauser_registry_address: Address,
        }

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

        pub async fn wait_for_responses(
            successful_responses: Arc<Mutex<usize>>,
            task_response_count: usize,
            timeout_duration: Duration,
        ) -> Result<Result<(), std::io::Error>, tokio::time::error::Elapsed> {
            tokio::time::timeout(timeout_duration, async move {
                loop {
                    let count = *successful_responses.lock().await;
                    if count >= task_response_count {
                        return Ok(());
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            })
            .await
        }
    }
}
