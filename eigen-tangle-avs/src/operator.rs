use alloy_contract::private::Ethereum;
use alloy_primitives::{Address, ChainId, Signature, B256, FixedBytes, ruint};
use alloy_provider::{Provider, RootProvider};
use alloy_signer_local::PrivateKeySigner;
use alloy_transport::BoxTransport;
use eigen_utils::avs_registry::reader::AvsRegistryChainReaderTrait;
use eigen_utils::avs_registry::writer::AvsRegistryChainWriterTrait;
use eigen_utils::avs_registry::AvsRegistryContractManager;
use eigen_utils::crypto::bls::{KeyPair, PrivateKey};
use eigen_utils::node_api::NodeApi;
use eigen_utils::types::AvsError;
use eigen_utils::Config;
use gadget_common::sp_core::testing::ECDSA;
use gadget_common::subxt_signer::SecretString;
use k256::ecdsa::{SigningKey, VerifyingKey};
use k256::elliptic_curve::SecretKey;
use k256::{Secp256k1, U256};
use log::error;
use rand_core::OsRng;
use ruint::aliases;
use sp_keystore::Keystore;
use std::env;
use std::future::Future;
use std::io::Read;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::{Bytes, FromStr};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use gadget_common::subxt_signer::bip39::rand;
use gadget_common::subxt_signer::bip39::rand::Rng;
use gadget_common::tangle_runtime::api::evm::calls::types::create2::Salt;

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
    WalletError(#[from] alloy_signer_local::LocalSignerError),
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
    // bls_keypair: KeyPair,
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
pub struct EigenTangleProvider {
    pub provider: RootProvider<BoxTransport, Ethereum>,
}

impl Provider for EigenTangleProvider {
    fn root(&self) -> &RootProvider<BoxTransport, Ethereum> {
        println!("Provider Root TEST");
        &self.provider
    }
}

#[derive(Clone)]
pub struct EigenTangleSigner {
    pub signer: PrivateKeySigner,
}

impl alloy_signer::Signer for EigenTangleSigner {
    fn sign_hash<'life0, 'life1, 'async_trait>(
        &'life0 self,
        hash: &'life1 B256,
    ) -> Pin<Box<dyn Future<Output = alloy_signer::Result<Signature>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        let signer = self.signer.clone();

        let signature_future = async move { signer.sign_hash(hash).await };

        Box::pin(signature_future)
    }

    fn address(&self) -> Address {
        println!("ADDRESS TEST");
        panic!("Signer functions for EigenTangleSigner are not yet implemented")
    }

    fn chain_id(&self) -> Option<ChainId> {
        println!("CHAIN ID TEST");
        panic!("Signer functions for EigenTangleSigner are not yet implemented")
    }

    fn set_chain_id(&mut self, _chain_id: Option<ChainId>) {
        println!("SET CHAIN ID TEST");
        panic!("Signer functions for EigenTangleSigner are not yet implemented")
    }
}

impl Config for NodeConfig {
    type TH = BoxTransport;
    type TW = BoxTransport;
    type PH = EigenTangleProvider;
    type PW = EigenTangleProvider;
    type S = EigenTangleSigner;
}

// impl std::fmt::Debug for EigenTangleSigner {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{:?}", self.signing_key.to_bytes())
//     }
// }

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

        log::warn!("About to read BLS key");
        let bls_key_password =
            std::env::var("OPERATOR_BLS_KEY_PASSWORD").unwrap_or_else(|_| "".to_string());
        let bls_keypair = KeyPair::read_private_key_from_file(
            &config.bls_private_key_store_path,
            &bls_key_password,
        )
        .map_err(OperatorError::from)?;

        // let chain_id = eth_client_http
        //     .get_chain_id()
        //     .await
        //     .map_err(|e| OperatorError::ChainIdError(e.to_string()))?;

        log::warn!("About to read ECDSA key");
        let ecdsa_key_password =
            std::env::var("OPERATOR_ECDSA_KEY_PASSWORD").unwrap_or_else(|_| "".to_string());
        let ecdsa_secret_key = eigen_utils::crypto::ecdsa::read_key(
            &config.ecdsa_private_key_store_path,
            &ecdsa_key_password,
        )
        .unwrap();
        let ecdsa_signing_key = SigningKey::from(&ecdsa_secret_key);

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

        log::info!("About to build AVS Registry Contract Manager");
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

        log::info!("About to get operator address");
        let operator_addr = Address::from_str(&config.operator_address)
            .map_err(|err| OperatorError::OperatorAddressError(err.to_string()))?;
        log::info!("About to get operator id");
        let operator_id = avs_registry_contract_manager
            .get_operator_id(operator_addr)
            .await?;

        log::info!("About to get service manager address");
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

        let mut salt = [0u8; 32];
        rand::thread_rng().fill(&mut salt);
        let sig_salt = FixedBytes::from_slice(&salt);
        let expiry = aliases::U256::from(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 3600);
        let register_result = avs_registry_contract_manager
            .register_operator_in_quorum_with_avs_registry_coordinator(
                &ecdsa_signing_key,
                sig_salt,
                expiry,
                &bls_keypair,
                alloy_primitives::Bytes::from(b"0"),
                "33125".to_string(),
            )
            .await;
        log::info!("Register result: {:?}", register_result);

        let operator = Operator {
            config: config.clone(),
            node_api,
            avs_registry_contract_manager,
            operator_id: [0u8; 32],
            operator_addr,
            tangle_validator_service_manager_addr,
        };

        log::info!(
            "Operator info: operatorId={}, operatorAddr={}, operatorG1Pubkey={:?}, operatorG2Pubkey={:?}",
            hex::encode(operator_id),
            config.operator_address,
            bls_keypair.get_pub_key_g1(),
            bls_keypair.get_pub_key_g2(),
        );

        log::info!("Operator Returning");

        Ok(operator)
    }

    pub async fn start(&self) -> Result<(), OperatorError> {
        log::info!("Starting operator.");
        let operator_is_registered = self
            .avs_registry_contract_manager
            .is_operator_registered(self.operator_addr)
            .await;
        log::info!("Operator registration status: {:?}", operator_is_registered);
        // if !operator_is_registered? {
        //     return Err(OperatorError::OperatorNotRegistered);
        // }

        // if self.config.enable_node_api {
        //     self.node_api.start(Default::default()).await?;
        // }

        gadget_executor::run_tangle_validator().await.unwrap();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;
    use super::*;
    use alloy_provider::ProviderBuilder;
    use alloy_transport_ws::WsConnect;
    use eigen_utils::crypto::bls::KeyPair;
    use k256::ecdsa::VerifyingKey;
    use k256::elliptic_curve::SecretKey;
    use rand_core::OsRng;

    static BLS_PASSWORD: &str = "BLS_PASSWORD";
    static ECDSA_PASSWORD: &str = "ECDSA_PASSWORD";

    // // --------- IMPORTS FOR ANVIL TESTS ---------
    // use alloy::signers::Signer;
    // use alloy_primitives::hex::FromHex;
    // use alloy_primitives::{address, ruint, Address, Bytes, TxKind, U256};
    // use alloy_provider::network::{EthereumWallet, ReceiptResponse, TransactionBuilder};
    // use alloy_provider::{Provider, WalletProvider};
    // use alloy_rpc_types_eth::{BlockId, BlockTransactions, TransactionRequest};
    // use alloy_signer_local::PrivateKeySigner;
    // use alloy_sol_types::sol;
    // use anvil::{spawn, NodeConfig};
    // // --------- IMPORTS FOR ANVIL TESTS ---------
    async fn test_anvil() {
        // // Initialize the logger
        // env_logger::init();
        //
        // let (api, mut handle) = spawn(NodeConfig::test().with_port(33125)).await;
        // api.anvil_auto_impersonate_account(true).await.unwrap();
        // let provider = handle.http_provider();
        //
        // let accounts = handle.dev_wallets().collect::<Vec<_>>();
        // let from = accounts[0].address();
        // let to = accounts[1].address();
        //
        // let amount = handle
        //     .genesis_balance()
        //     .checked_div(alloy_primitives::U256::from(2u64))
        //     .unwrap();
        //
        // let gas_price = provider.get_gas_price().await.unwrap();
        //
        // println!("Deploying Registry Coordinator...");
        //
        // let rc = RegistryCoordinator::deploy(
        //     provider.clone(),
        //     Address::from(address!("23e42f117e8643cc0174197c6c7cb38d8e5bd286")),
        //     Address::from(address!("33e423a17e86433a0174197c6c7cb38d8e3ad287")),
        //     Address::from(address!("43e42f117e8643cc03a4197c6c3ab38d8e5bd288")),
        //     Address::from(address!("53e42f117e8643cc01741973ac7cb3ad8e5bd289")),
        // )
        //     .await
        //     .unwrap();
        // let registry_coordinator_addr = rc.address();
        // println!("Registry Coordinator returned");
        // api.mine_one().await;
        // println!("Registry Coordinator deployed at: {:?}", registry_coordinator_addr);
        //
        // let dm = DelegationManager::deploy(
        //     provider.clone(),
        //     Address::from(address!("63e423a17e86433a0174197c6c7cb38d8e3ad280")),
        //     Address::from(address!("73e42f117e8643cc03a4197c6c3ab38d8e5bd281")),
        //     Address::from(address!("83e42f117e8643cc01741973ac7cb3ad8e5bd282")),
        // )
        //     .await
        //     .unwrap();
        // let delegation_manager_addr = dm.address();
        // println!("Delegation Manager returned");
        // api.mine_one().await;
        // println!("Delegation Manager deployed at: {:?}", delegation_manager_addr);
        //
        // let ad = AVSDirectory::deploy(
        //     provider.clone(),
        //     delegation_manager_addr.clone(),
        // )
        //     .await
        //     .unwrap();
        // let avs_dir_addr = ad.address();
        // println!("AVS Directory returned");
        // api.mine_one().await;
        // println!("AVS Directory deployed at: {:?}", avs_dir_addr);
        //
        // // get the block, await receipts
        // let block = provider
        //     .get_block(BlockId::latest(), false.into())
        //     .await
        //     .unwrap()
        //     .unwrap();
        //
        // let serv = handle.servers.pop().unwrap();
        // let res = serv.await.unwrap();
        // res.unwrap();
    }

    #[tokio::test]
    async fn test_generate_keys() {
        env_logger::init();

        // ---------------- BLS ----------------
        let bls_pair = KeyPair::gen_random().unwrap();
        bls_pair
            .save_to_file("./keystore/bls", BLS_PASSWORD)
            .unwrap();
        let bls_keys = KeyPair::read_private_key_from_file("./keystore/bls", BLS_PASSWORD).unwrap();
        assert_eq!(bls_pair.priv_key.key, bls_keys.priv_key.key);
        assert_eq!(bls_pair.pub_key, bls_keys.pub_key);

        //---------------- ECDSA ----------------
        let signing_key = SigningKey::random(&mut OsRng);
        let secret_key = SecretKey::from(signing_key.clone());
        let public_key = secret_key.public_key();
        let verifying_key = VerifyingKey::from(&signing_key);
        eigen_utils::crypto::ecdsa::write_key("./keystore/ecdsa", &secret_key, ECDSA_PASSWORD)
            .unwrap();

        let read_ecdsa_secret_key =
            eigen_utils::crypto::ecdsa::read_key("./keystore/ecdsa", ECDSA_PASSWORD).unwrap();
        let read_ecdsa_public_key = read_ecdsa_secret_key.public_key();
        let read_ecdsa_signing_key = SigningKey::from(&read_ecdsa_secret_key);
        let read_ecdsa_verifying_key = VerifyingKey::from(&read_ecdsa_signing_key);

        assert_eq!(secret_key, read_ecdsa_secret_key);
        assert_eq!(public_key, read_ecdsa_public_key);
        assert_eq!(signing_key, read_ecdsa_signing_key);
        assert_eq!(verifying_key, read_ecdsa_verifying_key);
    }

    #[tokio::test]
    async fn test_run_operator() {
        env_logger::init();
        let http_endpoint = "http://127.0.0.1:33125";
        let ws_endpoint = "ws://127.0.0.1:33125";
        let node_config = NodeConfig {
            node_api_ip_port_address: "127.0.0.1:9808".to_string(),
            eth_rpc_url: http_endpoint.to_string(),
            eth_ws_url: ws_endpoint.to_string(),
            bls_private_key_store_path: "./keystore/bls".to_string(),
            ecdsa_private_key_store_path: "./keystore/ecdsa".to_string(),
            avs_registry_coordinator_address: "0x5fbdb2315678afecb367f032d93f642f64180aa3"
                .to_string(),
            operator_state_retriever_address: "0x0000000000000000000000000000000000000002"
                .to_string(),
            eigen_metrics_ip_port_address: "127.0.0.1:9100".to_string(),
            tangle_validator_service_manager_address: "0x23e42f117e8643cc0174197c6c7cb38d8e5bd286"
                .to_string(),
            delegation_manager_address: "0xe7f1725e7734ce288f8367e1bb143e90bb3f0512".to_string(),
            avs_directory_address: "0x9fe46736679d2d9a65f0992f2272de9f3c7fa6e0".to_string(),
            operator_address: "0x0000000000000000000000000000000000000006".to_string(),
            enable_metrics: false,
            enable_node_api: false,
        };

        let signer = EigenTangleSigner {
            signer: PrivateKeySigner::random(),
        };

        let http_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .on_hyper_http(http_endpoint.parse().unwrap())
            .root()
            .clone()
            .boxed();

        let ws_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .on_ws(WsConnect::new(ws_endpoint))
            .await
            .unwrap()
            .root()
            .clone()
            .boxed();

        let operator = Operator::<NodeConfig>::new_from_config(
            node_config.clone(),
            EigenTangleProvider {
                provider: http_provider,
            },
            EigenTangleProvider {
                provider: ws_provider,
            },
            signer,
        )
        .await
        .unwrap();

        operator.start().await.unwrap();
    }
}
