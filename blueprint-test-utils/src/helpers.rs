use alloy_contract::{CallBuilder, CallDecoder};
use alloy_rpc_types::TransactionReceipt;
use futures::StreamExt;
use gadget_sdk::config::protocol::{EigenlayerContractAddresses, SymbioticContractAddresses};
use gadget_sdk::events_watcher::evm::{get_provider_http, get_provider_ws};
use gadget_sdk::{error, info};
use lazy_static::lazy_static;
use std::env;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::{collections::HashMap, time::Duration};
use tokio::process::Child;
use tokio::sync::Mutex;
use url::Url;

use crate::test_ext::{find_open_tcp_bind_port, NAME_IDS};
use alloy_primitives::{address, Address, Bytes, U256};
use alloy_provider::{network::Ethereum, Provider};
use alloy_transport::{Transport, TransportError};
use gadget_io::SupportedChains;
use gadget_sdk::config::Protocol;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlueprintError {
    #[error("Transport error occurred: {0}")]
    TransportError(#[from] TransportError),

    #[error("Contract error occurred: {0}")]
    ContractError(#[from] alloy_contract::Error),
}

alloy_sol_types::sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug)]
    IncredibleSquaringTaskManager,
    "./../blueprints/incredible-squaring-eigenlayer/contracts/out/IncredibleSquaringTaskManager.sol/IncredibleSquaringTaskManager.json"
);

alloy_sol_types::sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug)]
    PauserRegistry,
    "./../blueprints/incredible-squaring-eigenlayer/contracts/out/IPauserRegistry.sol/IPauserRegistry.json"
);

alloy_sol_types::sol!(
    #[allow(missing_docs, clippy::too_many_arguments)]
    #[sol(rpc)]
    #[derive(Debug)]
    RegistryCoordinator,
    "./../blueprints/incredible-squaring-eigenlayer/contracts/out/RegistryCoordinator.sol/RegistryCoordinator.json"
);

lazy_static! {
    pub static ref REGISTRY_COORDINATOR_ADDRESS: Address = env::var("REGISTRY_COORDINATOR_ADDRESS")
        .map(|addr| addr.parse().expect("Invalid REGISTRY_COORDINATOR_ADDRESS"))
        .unwrap_or_else(|_| address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690"));
    pub static ref OPERATOR_STATE_RETRIEVER_ADDRESS: Address =
        env::var("OPERATOR_STATE_RETRIEVER_ADDRESS")
            .map(|addr| addr
                .parse()
                .expect("Invalid OPERATOR_STATE_RETRIEVER_ADDRESS"))
            .unwrap_or_else(|_| address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8"));
    pub static ref DELEGATION_MANAGER_ADDRESS: Address = env::var("DELEGATION_MANAGER_ADDRESS")
        .map(|addr| addr.parse().expect("Invalid DELEGATION_MANAGER_ADDRESS"))
        .unwrap_or_else(|_| address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9"));
    pub static ref STRATEGY_MANAGER_ADDRESS: Address = env::var("STRATEGY_MANAGER_ADDRESS")
        .map(|addr| addr.parse().expect("Invalid STRATEGY_MANAGER_ADDRESS"))
        .unwrap_or_else(|_| address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707"));
    pub static ref AVS_DIRECTORY_ADDRESS: Address = env::var("AVS_DIRECTORY_ADDRESS")
        .map(|addr| addr.parse().expect("Invalid AVS_DIRECTORY_ADDRESS"))
        .unwrap_or_else(|_| address!("0000000000000000000000000000000000000000"));
    pub static ref TASK_MANAGER_ADDRESS: Address = env::var("TASK_MANAGER_ADDRESS")
        .map(|addr| addr.parse().expect("Invalid TASK_MANAGER_ADDRESS"))
        .unwrap_or_else(|_| address!("0000000000000000000000000000000000000000"));
}

pub struct EigenlayerTestEnvironment {
    pub http_endpoint: String,
    pub ws_endpoint: String,
    pub accounts: Vec<Address>,
    pub registry_coordinator_address: Address,
    pub operator_state_retriever_address: Address,
    pub delegation_manager_address: Address,
    pub strategy_manager_address: Address,
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
        registry_coordinator_address,
        operator_state_retriever_address,
        delegation_manager_address,
        strategy_manager_address,
        pauser_registry_address,
    }
}

pub struct BlueprintProcess {
    pub handle: Child,
    pub arguments: Vec<String>,
    pub env_vars: HashMap<String, String>,
}

impl BlueprintProcess {
    pub async fn new(
        program_path: PathBuf,
        arguments: Vec<String>,
        env_vars: HashMap<String, String>,
    ) -> Result<Self, std::io::Error> {
        let mut command = tokio::process::Command::new(program_path);
        command
            .args(&arguments)
            .envs(&env_vars)
            .kill_on_drop(true)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .stdin(std::process::Stdio::null());

        let handle = command.spawn()?;

        Ok(BlueprintProcess {
            handle,
            arguments,
            env_vars,
        })
    }

    pub async fn kill(&mut self) -> Result<(), std::io::Error> {
        self.handle.kill().await
    }
}

pub struct BlueprintProcessManager {
    processes: Arc<Mutex<Vec<BlueprintProcess>>>,
}

impl Default for BlueprintProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BlueprintProcessManager {
    pub fn new() -> Self {
        BlueprintProcessManager {
            processes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Helper function to start a blueprint process with given parameters.
    async fn start_blueprint_process(
        program_path: PathBuf,
        instance_id: usize,
        http_endpoint: &str,
        ws_endpoint: &str,
        protocol: Protocol,
    ) -> Result<BlueprintProcess, std::io::Error> {
        let tmp_store = uuid::Uuid::new_v4().to_string();
        let keystore_uri = format!(
            "./target/keystores/{}/{tmp_store}/",
            NAME_IDS[instance_id].to_lowercase()
        );
        assert!(
            !std::path::Path::new(&keystore_uri).exists(),
            "Keystore URI cannot exist: {}",
            keystore_uri
        );

        let keystore_uri_normalized =
            std::path::absolute(&keystore_uri).expect("Failed to resolve keystore URI");
        let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());

        let mut arguments = vec![
            "run".to_string(),
            format!("--bind-addr={}", IpAddr::from_str("127.0.0.1").unwrap()),
            format!("--bind-port={}", find_open_tcp_bind_port()),
            format!("--http-rpc-url={}", Url::parse(http_endpoint).unwrap()),
            format!("--ws-rpc-url={}", Url::parse(ws_endpoint).unwrap()),
            format!("--keystore-uri={}", keystore_uri_str.clone()),
            format!("--chain={}", SupportedChains::LocalTestnet),
            format!("--verbose={}", 3),
            format!("--pretty={}", true),
            format!("--blueprint-id={}", instance_id),
            format!("--service-id={}", instance_id),
            format!("--protocol={}", protocol),
        ];

        match protocol {
            Protocol::Tangle => {
                arguments.push(format!("--blueprint-id={}", instance_id));
                arguments.push(format!("--service-id={}", instance_id));
            }
            Protocol::Eigenlayer => {
                arguments.push(format!(
                    "--registry-coordinator-address={}",
                    EigenlayerContractAddresses::default().registry_coordinator_address
                ));
                arguments.push(format!(
                    "--operator-state-retriever-address={}",
                    EigenlayerContractAddresses::default().operator_state_retriever_address
                ));
                arguments.push(format!(
                    "--delegation-manager-address={}",
                    EigenlayerContractAddresses::default().delegation_manager_address
                ));
                arguments.push(format!(
                    "--strategy-manager-address={}",
                    EigenlayerContractAddresses::default().strategy_manager_address
                ));
                arguments.push(format!(
                    "--avs-directory-address={}",
                    EigenlayerContractAddresses::default().avs_directory_address
                ));
            }
            Protocol::Symbiotic => {
                arguments.push(format!(
                    "--operator-registry-address={}",
                    SymbioticContractAddresses::default().operator_registry_address
                ));
                arguments.push(format!(
                    "--network-registry-address={}",
                    SymbioticContractAddresses::default().network_registry_address
                ));
                arguments.push(format!(
                    "--base-delegator-address={}",
                    SymbioticContractAddresses::default().base_delegator_address
                ));
                arguments.push(format!(
                    "--network-opt-in-service-address={}",
                    SymbioticContractAddresses::default().network_opt_in_service_address
                ));
                arguments.push(format!(
                    "--vault-opt-in-service-address={}",
                    SymbioticContractAddresses::default().vault_opt_in_service_address
                ));
                arguments.push(format!(
                    "--slasher-address={}",
                    SymbioticContractAddresses::default().slasher_address
                ));
                arguments.push(format!(
                    "--veto-slasher-address={}",
                    SymbioticContractAddresses::default().veto_slasher_address
                ));
            }
        }

        let mut env_vars = HashMap::new();
        env_vars.insert("HTTP_RPC_URL".to_string(), http_endpoint.to_string());
        env_vars.insert("WS_RPC_URL".to_string(), ws_endpoint.to_string());
        env_vars.insert("KEYSTORE_URI".to_string(), keystore_uri_str.clone());
        env_vars.insert("DATA_DIR".to_string(), keystore_uri_str);
        env_vars.insert("REGISTRATION_MODE_ON".to_string(), "true".to_string());

        BlueprintProcess::new(program_path, arguments, env_vars).await
    }

    /// Starts multiple blueprint processes and adds them to the process manager.
    pub async fn start_blueprints(
        &self,
        blueprint_paths: Vec<PathBuf>,
        http_endpoint: &str,
        ws_endpoint: &str,
        protocol: Protocol,
    ) -> Result<(), std::io::Error> {
        for (index, program_path) in blueprint_paths.into_iter().enumerate() {
            let process = Self::start_blueprint_process(
                program_path,
                index,
                http_endpoint,
                ws_endpoint,
                protocol,
            )
            .await?;
            self.processes.lock().await.push(process);
        }
        Ok(())
    }

    pub async fn kill_all(&self) -> Result<(), std::io::Error> {
        let mut processes = self.processes.lock().await;
        for process in processes.iter_mut() {
            process.kill().await?;
        }
        processes.clear();
        Ok(())
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

pub async fn get_receipt<T, P, D>(
    call: CallBuilder<T, P, D, Ethereum>,
) -> Result<TransactionReceipt, BlueprintError>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum>,
    D: CallDecoder,
{
    let pending_tx = match call.send().await {
        Ok(tx) => tx,
        Err(e) => {
            error!("Failed to send transaction: {:?}", e);
            return Err(e.into());
        }
    };

    let receipt = match pending_tx.get_receipt().await {
        Ok(receipt) => receipt,
        Err(e) => {
            error!("Failed to get transaction receipt: {:?}", e);
            return Err(e.into());
        }
    };

    Ok(receipt)
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
