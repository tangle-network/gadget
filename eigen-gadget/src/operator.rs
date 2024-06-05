use alloy_primitives::{Address, U256};
use alloy_core::{core::U256, bls, txmgr, wallet};
use alloy_provider::network::Ethereum;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_transport::Transport;
use aws_sdk_kms::Client as KmsClient;
use bls::KeyPair;
use eigen_utils::avs_registry::subscriber::AvsRegistryChainSubscriber;
use log::{error, info};
use prometheus::Registry;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use thiserror::Error;

const AVS_NAME: &str = "incredible-squaring";
const SEM_VER: &str = "0.0.1";

#[derive(Debug, Error)]
pub enum OperatorError {
    #[error("Cannot create HTTP ethclient: {0}")]
    HttpEthClientError(String),
    #[error("Cannot create WS ethclient: {0}")]
    WsEthClientError(String),
    #[error("Cannot parse BLS private key: {0}")]
    BlsPrivateKeyError(String),
    #[error("Cannot get chainId: {0}")]
    ChainIdError(String),
    #[error("Error creating AvsWriter: {0}")]
    AvsWriterError(String),
    #[error("Error creating AvsReader: {0}")]
    AvsReaderError(String),
    #[error("Error creating AvsSubscriber: {0}")]
    AvsSubscriberError(String),
    #[error("Cannot create AggregatorRpcClient: {0}")]
    AggregatorRpcClientError(String),
    #[error("Cannot get operator id: {0}")]
    OperatorIdError(String),
    #[error("Operator is not registered. Register using the operator-cli before starting operator.")]
    OperatorNotRegistered,
    #[error("Error in metrics server: {0}")]
    MetricsServerError(String),
    #[error("Error in websocket subscription: {0}")]
    WebsocketSubscriptionError(String),
    #[error("Error getting task response header hash: {0}")]
    TaskResponseHeaderHashError(String),
}

pub struct Operator<T, P>
where
    T: Transport,
    P: Provider<T, Ethereum>,
{
    config: NodeConfig,
    eth_client: P,
    // metrics_reg: Registry,
    // metrics: Metrics,
    node_api: NodeApi,
    avs_writer: AvsWriter,
    avs_reader: Arc<dyn AvsReaderer>,
    avs_subscriber: AvsRegistryChainSubscriber<T, ProviderBuilder<>>,
    eigenlayer_reader: Arc<dyn ElReader>,
    eigenlayer_writer: Arc<dyn ElWriter>,
    bls_keypair: KeyPair,
    operator_id: [u8; 32],
    operator_addr: Address,
    new_task_created_chan: mpsc::Sender<IncredibleSquaringTaskManagerNewTaskCreated>,
    aggregator_server_ip_port_addr: String,
    aggregator_rpc_client: Arc<dyn AggregatorRpcClienter>,
    credible_squaring_service_manager_addr: Address,
}

#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub node_api_ip_port_address: String,
    pub eth_rpc_url: String,
    pub eth_ws_url: String,
    pub bls_private_key_store_path: String,
    pub ecdsa_private_key_store_path: String,
    pub avs_registry_coordinator_address: String,
    pub operator_state_retriever_address: String,
    pub eigen_metrics_ip_port_address: String,
    pub operator_address: String,
    pub enable_metrics: bool,
}


impl Operator {
    pub fn new_from_config(config: NodeConfig) -> Result<Self, OperatorError> {
        let metrics_reg = Registry::new();
        let avs_and_eigen_metrics = Metrics::new(AVS_NAME, eigen_metrics, &metrics_reg);

        let node_api = NodeApi::new(AVS_NAME, SEM_VER, &config.node_api_ip_port_address);

        let (eth_rpc_client, eth_ws_client) = if config.enable_metrics {
            let rpc_calls_collector = RpcCallsCollector::new(AVS_NAME, &metrics_reg);
            (
                EthClient::new_instrumented(&config.eth_rpc_url, rpc_calls_collector.clone())?,
                EthClient::new_instrumented(&config.eth_ws_url, rpc_calls_collector)?,
            )
        } else {
            (
                EthClient::new(&config.eth_rpc_url)?,
                EthClient::new(&config.eth_ws_url)?,
            )
        };

        let bls_key_password = std::env::var("OPERATOR_BLS_KEY_PASSWORD").unwrap_or_else(|_| "".to_string());
        let bls_keypair = KeyPair::read_private_key_from_file(&config.bls_private_key_store_path, &bls_key_password)?;

        let chain_id = eth_rpc_client.chain_id().map_err(|e| OperatorError::ChainIdError(e.to_string()))?;

        let ecdsa_key_password = std::env::var("OPERATOR_ECDSA_KEY_PASSWORD").unwrap_or_else(|_| "".to_string());
        let signer_v2 = SignerV2::from_config(SignerV2Config {
            keystore_path: config.ecdsa_private_key_store_path.clone(),
            password: ecdsa_key_password.clone(),
        }, chain_id)?;

        let chainio_config = BuildAllConfig {
            eth_http_url: config.eth_rpc_url.clone(),
            eth_ws_url: config.eth_ws_url.clone(),
            registry_coordinator_addr: config.avs_registry_coordinator_address,
            operator_state_retriever_addr: config.operator_state_retriever_address,
            avs_name: AVS_NAME.to_string(),
            prom_metrics_ip_port_address: config.eigen_metrics_ip_port_address.clone(),
        };

        let operator_ecdsa_private_key = EcdsaKey::read_key(&config.ecdsa_private_key_store_path, &ecdsa_key_password)?;
        let sdk_clients = Clients::build_all(chainio_config, operator_ecdsa_private_key.clone())?;

        let sk_wallet = Wallet::new_private_key_wallet(eth_rpc_client.clone(), signer_v2, config.operator_address.parse()?)?;
        let tx_mgr = TxMgr::new_simple_tx_manager(sk_wallet, eth_rpc_client.clone(), &logger, config.operator_address.parse()?);

        let avs_writer = AvsWriter::build(
            tx_mgr, config.avs_registry_coordinator_address.parse()?,
            config.operator_state_retriever_address.parse()?, eth_rpc_client.clone(), &logger,
        )?;

        let avs_reader = AvsReader::build(
            config.avs_registry_coordinator_address.parse()?,
            config.operator_state_retriever_address.parse()?,
            eth_rpc_client.clone())?;

        let avs_subscriber = AvsSubscriber::build(config.avs_registry_coordinator_address.parse()?,
                                                  config.operator_state_retriever_address.parse()?, eth_ws_client.clone())?;

        let quorum_names: HashMap<QuorumNum, String> = [(0, "quorum0".to_string())].iter().cloned().collect();
        let economic_metrics_collector = EconomicMetricsCollector::new(
            sdk_clients.el_chain_reader.clone(), sdk_clients.avs_registry_chain_reader.clone(),
            AVS_NAME, &logger, config.operator_address.parse()?, quorum_names);
        metrics_reg.register(Box::new(economic_metrics_collector))?;

        let aggregator_rpc_client = AggregatorRpcClient::new(&config.aggregator_server_ip_port_address, &logger, &avs_and_eigen_metrics)?;

        let mut operator = Operator {
            config: config.clone(),
            node_api,
            eth_client: eth_rpc_client,
            avs_writer,
            avs_reader: Arc::new(avs_reader),
            avs_subscriber: Arc::new(avs_subscriber),
            eigenlayer_reader: sdk_clients.el_chain_reader.clone(),
            eigenlayer_writer: sdk_clients.el_chain_writer.clone(),
            bls_keypair,
            operator_id: [0u8; 32], // this is set below
            operator_addr: config.operator_address.parse()?,
            new_task_created_chan: mpsc::channel(100).0,
            aggregator_server_ip_port_addr: config.aggregator_server_ip_port_address.clone(),
            aggregator_rpc_client: Arc::new(aggregator_rpc_client),
            credible_squaring_service_manager_addr: config.avs_registry_coordinator_address.parse()?,
        };

        if config.register_operator_on_startup {
            operator.register_operator_on_startup(operator_ecdsa_private_key, config.token_strategy_addr.parse()?);
        }

        let operator_id = sdk_clients.avs_registry_chain_reader.get_operator_id(&operator.operator_addr)?;
        operator.operator_id = operator_id;

        log::info!("Operator info: operatorId={}, operatorAddr={}, operatorG1Pubkey={}, operatorG2Pubkey={}",
            hex::encode(operator_id),
            config.operator_address,
            hex::encode(operator.bls_keypair.get_pub_key_g1()),
            hex::encode(operator.bls_keypair.get_pub_key_g2())
        );

        Ok(operator)
    }

    pub async fn start(&self) -> Result<(), OperatorError> {
        let operator_is_registered = self.avs_reader.is_operator_registered(&self.operator_addr)?;
        if !operator_is_registered {
            return Err(OperatorError::OperatorNotRegistered);
        }

        log::info!("Starting operator.");

        if self.config.enable_node_api {
            self.node_api.start();
        }

        let mut sub = self.avs_subscriber.subscribe_to_new_tasks(self.new_task_created_chan.clone());

        loop {
            tokio::select! {
                _ = ctx.done() => {
                    return Ok(());
                },
                Some(err) = metrics_err_chan.next() => {
                    log::fatal!(&format!("Error in metrics server: {}", err));
                },
                Some(err) = sub.err() => {
                    log::error!(&format!("Error in websocket subscription: {}", err));
                    sub.unsubscribe();
                    sub = self.avs_subscriber.subscribe_to_new_tasks(self.new_task_created_chan.clone());
                },
                Some(new_task_created_log) = self.new_task_created_chan.recv() => {
                    self.metrics.inc_num_tasks_received();
                    let task_response = self.process_new_task_created_log(&new_task_created_log);
                    if let Ok(signed_task_response) = self.sign_task_response(&task_response) {
                        tokio::spawn(self.aggregator_rpc_client.send_signed_task_response_to_aggregator(signed_task_response));
                    }
                },
            }
        }
    }

    fn process_new_task_created_log(&self, new_task_created_log: &IncredibleSquaringTaskManagerNewTaskCreated) -> IncredibleSquaringTaskManagerTaskResponse {
        log::debug!(&format!("Received new task: {:?}", new_task_created_log));
        log::info!(&format!("Received new task: numberToBeSquared={}, taskIndex={}, taskCreatedBlock={}, quorumNumbers={}, QuorumThresholdPercentage={}",
                                  new_task_created_log.task.number_to_be_squared,
                                  new_task_created_log.task_index,
                                  new_task_created_log.task.task_created_block,
                                  new_task_created_log.task.quorum_numbers,
                                  new_task_created_log.task.quorum_threshold_percentage));
        let number_squared = new_task_created_log.task.number_to_be_squared.pow(2);
        IncredibleSquaringTaskManagerTaskResponse {
            reference_task_index: new_task_created_log.task_index,
            number_squared,
        }
    }

    fn sign_task_response(&self, task_response: &IncredibleSquaringTaskManagerTaskResponse) -> Result<SignedTaskResponse, OperatorError> {
        let task_response_hash = core::get_task_response_digest(task_response)
            .map_err(|e| OperatorError::TaskResponseHeaderHashError(e.to_string()))?;
        let bls_signature = self.bls_keypair.sign_message(&task_response_hash);
        let signed_task_response = SignedTaskResponse {
            task_response: task_response.clone(),
            bls_signature,
            operator_id: self.operator_id,
        };
        log::debug!(&format!("Signed task response: {:?}", signed_task_response));
        Ok(signed_task_response)
    }
}
