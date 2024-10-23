use alloy_contract::{CallBuilder, CallDecoder};
use alloy_provider::{RootProvider, WsConnect};
use alloy_rpc_types::TransactionReceipt;
use futures::StreamExt;
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;
use url::Url;

use alloy_primitives::{address, Address, Bytes, U256};
use alloy_provider::{network::Ethereum, Provider, ProviderBuilder};
use alloy_transport::{BoxTransport, Transport, TransportResult};

use crate::anvil;
use crate::test_ext::NAME_IDS;
use gadget_io::SupportedChains;
use gadget_sdk::config::Protocol;

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
pub struct EigenlayerTestEnvironment {
    pub http_endpoint: String,
    pub ws_endpoint: String,
    pub accounts: Vec<Address>,
    pub registry_coordinator_address: Address,
    pub operator_state_retriever_address: Address,
    pub pauser_registry_address: Address,
}

pub fn get_provider_http(http_endpoint: &str) -> RootProvider<BoxTransport> {
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone()
        .boxed();

    provider
}

pub async fn get_provider_ws(ws_endpoint: &str) -> RootProvider<BoxTransport> {
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_ws(WsConnect::new(ws_endpoint))
        .await
        .unwrap()
        .root()
        .clone()
        .boxed();

    provider
}

pub async fn setup_eigenlayer_test_environment(
    anvil_state_path: &str,
) -> EigenlayerTestEnvironment {
    let (_container, http_endpoint, ws_endpoint) =
        anvil::start_anvil_container(anvil_state_path, true).await;

    std::env::set_var("EIGENLAYER_HTTP_ENDPOINT", http_endpoint.clone());
    std::env::set_var("EIGENLAYER_WS_ENDPOINT", ws_endpoint.clone());

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let provider = Arc::new(
        ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(http_endpoint.parse().unwrap())
            .root()
            .clone()
            .boxed(),
    );

    let accounts = provider.get_accounts().await.unwrap();

    let registry_coordinator_address = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
    let operator_state_retriever_address = address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
    let erc20_mock_address = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");

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
    let _receipt = get_receipt(registry_coordinator.createQuorum(
        operator_set_params,
        0,
        vec![strategy_params],
    ))
    .await
    .unwrap();

    EigenlayerTestEnvironment {
        http_endpoint,
        ws_endpoint,
        accounts,
        registry_coordinator_address,
        operator_state_retriever_address,
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

impl BlueprintProcessManager {
    pub fn new() -> Self {
        BlueprintProcessManager {
            processes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn start_blueprint(
        &self,
        program_path: PathBuf,
        instance_id: usize,
        http_rpc_url: &str,
        ws_rpc_url: &str,
    ) -> Result<(), std::io::Error> {
        let tmp_store = uuid::Uuid::new_v4().to_string();
        let keystore_uri = PathBuf::from(format!(
            "./target/keystores/{}/{tmp_store}/",
            NAME_IDS[instance_id].to_lowercase()
        ));
        assert!(
            !keystore_uri.exists(),
            "Keystore URI cannot exist: {}",
            keystore_uri.display()
        );

        let keystore_uri_normalized =
            std::path::absolute(keystore_uri).expect("Failed to resolve keystore URI");
        let keystore_uri_str = format!("file:{}", keystore_uri_normalized.display());

        let arguments = vec![
            "run".to_string(),
            format!("--bind-addr={}", IpAddr::from_str("127.0.0.1").unwrap()),
            format!("--bind-port={}", 8545u16 + instance_id as u16),
            format!("--http-url={}", Url::parse(http_rpc_url).unwrap()),
            format!("--ws-url={}", Url::parse(ws_rpc_url).unwrap()),
            format!("--keystore-uri={}", keystore_uri_str.clone()),
            format!("--chain={}", SupportedChains::LocalTestnet),
            format!("--verbose={}", 3),
            format!("--pretty={}", true),
            format!("--blueprint-id={}", instance_id),
            format!("--service-id={}", instance_id),
            format!("--protocol={}", Protocol::Eigenlayer),
        ];

        let mut env_vars = HashMap::new();
        env_vars.insert("HTTP_RPC_URL".to_string(), http_rpc_url.to_string());
        env_vars.insert("WS_RPC_URL".to_string(), ws_rpc_url.to_string());
        env_vars.insert("KEYSTORE_URI".to_string(), keystore_uri_str.clone());
        env_vars.insert("DATA_DIR".to_string(), keystore_uri_str);
        env_vars.insert("BLUEPRINT_ID".to_string(), instance_id.to_string());
        env_vars.insert("SERVICE_ID".to_string(), instance_id.to_string());
        env_vars.insert("REGISTRATION_MODE_ON".to_string(), "true".to_string());
        env_vars.insert(
            "OPERATOR_BLS_KEY_PASSWORD".to_string(),
            "BLS_PASSWORD".to_string(),
        );
        env_vars.insert(
            "OPERATOR_ECDSA_KEY_PASSWORD".to_string(),
            "ECDSA_PASSWORD".to_string(),
        );

        let process = BlueprintProcess::new(program_path, arguments, env_vars).await?;
        self.processes.lock().await.push(process);
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

pub async fn deploy_task_manager<T: Transport + Clone>(
    http_endpoint: &str,
    registry_coordinator_address: Address,
    pauser_registry_address: Address,
    task_generator_address: Address,
    accounts: &[Address],
) -> Address {
    let provider = get_provider_http(http_endpoint);
    let deploy_call = IncredibleSquaringTaskManager::deploy_builder(
        provider.clone(),
        registry_coordinator_address,
        10u32,
    );
    let task_manager_address = get_receipt(deploy_call)
        .await
        .unwrap()
        .contract_address
        .unwrap();

    std::env::set_var("TASK_MANAGER_ADDRESS", task_manager_address.to_string());

    let task_manager = IncredibleSquaringTaskManager::new(task_manager_address, provider.clone());
    // Initialize the Incredible Squaring Task Manager
    let init_call = task_manager.initialize(
        pauser_registry_address,
        accounts[1],
        accounts[9],
        task_generator_address,
    );
    let init_receipt = get_receipt(init_call).await.unwrap();
    assert!(init_receipt.status());

    task_manager_address
}

pub async fn setup_task_spawner<T: Transport + Clone>(
    task_manager_address: Address,
    registry_coordinator_address: Address,
    task_generator_address: Address,
    accounts: &[Address],
    http_endpoint: String,
) -> impl std::future::Future<Output = ()> {
    let provider = get_provider_http(http_endpoint.as_str());
    let task_manager = IncredibleSquaringTaskManager::new(task_manager_address, provider.clone());
    let registry_coordinator =
        RegistryCoordinator::new(registry_coordinator_address, provider.clone());

    let operators = vec![vec![accounts[0]]];
    let quorums = Bytes::from(vec![0]);

    async move {
        let mut task_count = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(10000)).await;

            if get_receipt(
                task_manager
                    .createNewTask(U256::from(2), 100u32, Bytes::from(vec![0]))
                    .from(task_generator_address),
            )
            .await
            .unwrap()
            .status()
            {
                log::info!("Deployed a new task");
                task_count += 1;
            }

            if get_receipt(
                registry_coordinator.updateOperatorsForQuorum(operators.clone(), quorums.clone()),
            )
            .await
            .unwrap()
            .status()
            {
                log::info!("Updated operators for quorum 0");
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
            log::info!("Mined a block...");

            if task_count >= 5 {
                break;
            }
        }
    }
}

pub async fn setup_task_response_listener<T: Transport + Clone>(
    task_manager_address: Address,
    ws_endpoint: &str,
    successful_responses: Arc<Mutex<usize>>,
) -> impl std::future::Future<Output = ()> {
    let task_manager = IncredibleSquaringTaskManager::new(
        task_manager_address,
        get_provider_ws(ws_endpoint).await,
    );
    let ws_provider = ProviderBuilder::new()
        .on_ws(alloy_provider::WsConnect::new(ws_endpoint))
        .await
        .unwrap()
        .root()
        .clone()
        .boxed();

    async move {
        let filter = task_manager.TaskResponded_filter().filter;
        let mut event_stream = match ws_provider.subscribe_logs(&filter).await {
            Ok(stream) => stream.into_stream(),
            Err(e) => {
                log::error!("Failed to subscribe to logs: {:?}", e);
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
            if *counter >= 1 {
                break;
            }
        }
    }
}

pub async fn get_receipt<T, P, D>(
    call: CallBuilder<T, P, D, Ethereum>,
) -> TransportResult<TransactionReceipt>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum>,
    D: CallDecoder,
{
    let pending_tx = call.send().await.unwrap();
    let receipt = pending_tx.get_receipt().await?;

    Ok(receipt)
}
