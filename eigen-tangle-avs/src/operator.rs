// use alloy_core::{bls, core::U256, txmgr, wallet};
use alloy_primitives::Address;
// use alloy_provider::network::Ethereum;
// use alloy_provider::{Provider, ProviderBuilder};
// use alloy_rpc_client::WsConnect;
// use alloy_signer_wallet::Wallet;
// use alloy_transport::Transport;
// use aws_sdk_kms::Client as KmsClient;
// use bls::KeyPair;
// use eigen_utils::avs_registry::subscriber::AvsRegistryChainSubscriberTrait;
use eigen_utils::crypto::bls::KeyPair;
// use eigen_utils::el_contracts::reader::ElReader;
// use eigen_utils::el_contracts::writer::ElWriter;
use eigen_utils::node_api::NodeApi;
// use eigen_utils::services::bls_aggregation::SignedTaskResponseDigest;
use eigen_utils::types::AvsError;
use log::error;
// use prometheus::Registry;
// use reqwest::Url;
// use std::collections::HashMap;
// use std::env;
// use std::marker::PhantomData;
// use std::path::PathBuf;
use std::str::FromStr;
// use std::sync::Arc;
// use std::time::Duration;
use thiserror::Error;
// use tokio::sync::mpsc;
use eigen_utils::avs_registry::reader::AvsRegistryChainReaderTrait;
use eigen_utils::avs_registry::AvsRegistryContractManager;
use eigen_utils::Config;
// use eigen_utils::services::operator_info::OperatorInfoServiceTrait;

// use crate::avs::reader::AvsReader;
// use crate::avs::subscriber::AvsSubscriber;
// use crate::avs::writer::AvsWriter;
// use crate::avs::TangleValidatorTaskManager;

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
    #[error("Error in Operator Address: {0}")]
    OperatorAddressError(String),
    #[error(
        "Operator is not registered. Register using the operator-cli before starting operator."
    )]
    OperatorNotRegistered,
    #[error("Error in metrics server: {0}")]
    MetricsServerError(String),
    #[error("Error in Service Manager Address: {0}")]
    ServiceManagerAddressError(String),
    #[error("Error in websocket subscription: {0}")]
    WebsocketSubscriptionError(String),
    #[error("AVS SDK error")]
    AvsSdkError(#[from] AvsError),
    #[error("Wallet error")]
    WalletError(#[from] alloy_signer_wallet::WalletError),
}

#[allow(dead_code)]
pub struct Operator<T: Config> {
    config: NodeConfig,
    // eth_client: P,
    // metrics_reg: Registry,
    // metrics: Metrics,
    node_api: NodeApi,
    avs_registry_contract_manager: AvsRegistryContractManager<T>,
    // tangle_validator_contract_manager: TangleValidatorContractManager<T>,
    // avs_writer: AvsWriter<T, P>,
    // avs_reader: AvsReader<T, P>,
    // avs_subscriber: AvsRegistryChainSubscriber<T, P>,
    // eigenlayer_reader: Arc<dyn ElReader<T, P>>,
    // eigenlayer_writer: Arc<dyn ElWriter>,
    bls_keypair: KeyPair,
    operator_id: [u8; 32],
    operator_addr: Address,
    tangle_validator_service_manager_addr: Address,
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
    pub tangle_validator_service_manager_address: String,
    pub delegation_manager_address: String,
    pub avs_directory_address: String,
    pub operator_address: String,
    pub enable_metrics: bool,
    pub enable_node_api: bool,
}

#[derive(Clone)]
pub struct TangleValidatorContractManager<T: Config> {
    pub task_manager_addr: Address,
    pub service_manager_addr: Address,
    pub eth_client_http: T::PH,
    pub eth_client_ws: T::PW,
    pub signer: T::S,
}

#[derive(Debug, Clone)]
pub struct SetupConfig<T: Config> {
    pub registry_coordinator_addr: Address,
    pub operator_state_retriever_addr: Address,
    pub delegate_manager_addr: Address,
    pub avs_directory_addr: Address,
    pub eth_client_http: T::PH,
    pub eth_client_ws: T::PW,
    pub signer: T::S,
}

impl<T: Config> Operator<T> {
    pub async fn new_from_config(
        config: NodeConfig,
        eth_client_http: T::PH,
        eth_client_ws: T::PW,
        // operator_info_service: I,
        signer: T::S,
    ) -> Result<Self, OperatorError> {
        // let metrics_reg = Registry::new();
        // let avs_and_eigen_metrics = Metrics::new(AVS_NAME, eigen_metrics, &metrics_reg);

        let node_api = NodeApi::new(AVS_NAME, SEM_VER, &config.node_api_ip_port_address);

        // let eth_rpc_client = ProviderBuilder::default()
        //     .with_recommended_fillers()
        //     .on_http(
        //         Url::parse(&config.eth_rpc_url)
        //             .map_err(|e| OperatorError::HttpEthClientError(e.to_string()))?,
        //     );
        // let eth_ws_client = ProviderBuilder::default()
        //     .with_recommended_fillers()
        //     .on_ws(WsConnect::new(&config.eth_ws_url))
        //     .await
        //     .map_err(|e| AvsError::from(e))?;

        let bls_key_password =
            std::env::var("OPERATOR_BLS_KEY_PASSWORD").unwrap_or_else(|_| "".to_string());
        let bls_keypair = KeyPair::read_private_key_from_file(
            &config.bls_private_key_store_path,
            &bls_key_password,
        )
        .map_err(OperatorError::from)?;

        // let chain_id = eth_rpc_client
        //     .get_chain_id()
        //     .await
        //     .map_err(|e| OperatorError::ChainIdError(e.to_string()))?;

        // let ecdsa_key_password =
        //     std::env::var("OPERATOR_ECDSA_KEY_PASSWORD").unwrap_or_else(|_| "".to_string());
        // let keystore_file_path = PathBuf::from(
        //     env::var("CARGO_MANIFEST_DIR").map_err(|e| AvsError::KeyError(e.to_string()))?,
        // )
        // .join(config.ecdsa_private_key_store_path);

        let setup_config = SetupConfig::<T> {
            registry_coordinator_addr: Address::from_str(&config.avs_registry_coordinator_address)
                .unwrap(),
            operator_state_retriever_addr: Address::from_str(
                &config.operator_state_retriever_address,
            )
            .unwrap(),
            delegate_manager_addr: Address::from_str(&config.delegation_manager_address).unwrap(),
            avs_directory_addr: Address::from_str(&config.avs_directory_address).unwrap(),
            eth_client_http: eth_client_http.clone(),
            eth_client_ws: eth_client_ws.clone(),
            signer: signer.clone(),
        };

        let avs_registry_contract_manager = AvsRegistryContractManager::build(
            Address::from_str(&config.tangle_validator_service_manager_address).unwrap(),
            setup_config.registry_coordinator_addr,
            setup_config.operator_state_retriever_addr,
            setup_config.delegate_manager_addr,
            setup_config.avs_directory_addr,
            eth_client_http.clone(),
            eth_client_ws.clone(),
            signer.clone(),
        )
        .await?;

        let operator_addr = Address::from_str(&config.operator_address)
            .map_err(|err| OperatorError::OperatorAddressError(err.to_string()))?;
        let operator_id = avs_registry_contract_manager
            .get_operator_id(operator_addr)
            .await?;
        let tangle_validator_service_manager_addr =
            Address::from_str(&config.tangle_validator_service_manager_address)
                .map_err(|err| OperatorError::ServiceManagerAddressError(err.to_string()))?;

        // let avs_writer = AvsWriter::build(
        //     &config.avs_registry_coordinator_address,
        //     &config.operator_state_retriever_address,
        //     eth_rpc_client.clone(),
        // )
        // .await?;
        //
        // let avs_reader = AvsReader::build(
        //     &config.avs_registry_coordinator_address,
        //     &config.operator_state_retriever_address,
        //     eth_rpc_client.clone(),
        // )
        // .await?;
        //
        // let avs_subscriber = AvsSubscriber::build(
        //     &config.avs_registry_coordinator_address,
        //     &config.operator_state_retriever_address,
        //     eth_ws_client.clone(),
        // )
        // .await?;

        // let tangle_validator_contract_manager = TangleValidatorContractManager::build(
        //     setup_config.registry_coordinator_addr,
        //     setup_config.operator_state_retriever_addr,
        //     eth_client_http.clone(),
        //     eth_client_ws.clone(),
        //     signer.clone(),
        // )
        //     .await?;

        let operator = Operator {
            config: config.clone(),
            node_api,
            // eth_client: eth_rpc_client,
            // avs_writer,
            // avs_reader: avs_reader,
            // avs_subscriber: avs_subscriber,
            // eigenlayer_reader: sdk_clients.el_chain_reader.clone(),
            // eigenlayer_writer: sdk_clients.el_chain_writer.clone(),
            avs_registry_contract_manager,
            // tangle_validator_contract_manager,
            bls_keypair,
            operator_id: [0u8; 32], // this is set below
            operator_addr,
            tangle_validator_service_manager_addr,
        };

        // if config.register_operator_on_startup {
        //     operator.register_operator_on_startup(
        //         operator_ecdsa_private_key,
        //         config.token_strategy_addr.parse()?,
        //     );
        // }

        // let operator_id = sdk_clients
        //     .avs_registry_chain_reader
        //     .get_operator_id(&operator.operator_addr)?;
        // operator.operator_id = operator_id;

        log::info!("Operator info: operatorId={}, operatorAddr={}, operatorG1Pubkey={}, operatorG2Pubkey={}",
            hex::encode(operator_id),
            config.operator_address,
            hex::encode(operator.bls_keypair.get_pub_key_g1().to_bytes()),
            hex::encode(operator.bls_keypair.get_pub_key_g2().to_bytes())
        );

        Ok(operator)
    }

    pub async fn start(&self) -> Result<(), OperatorError> {
        // let operator_is_registered = self
        //     .avs_reader
        //     .is_operator_registered(&self.operator_addr)?;
        // if !operator_is_registered {
        //     return Err(OperatorError::OperatorNotRegistered);
        // }

        log::info!("Starting operator.");

        // if self.config.enable_node_api {
        //     self.node_api.start(Default::default()).await?;
        // }

        // TODO: Run the executor thing.

        Ok(())
    }
}
