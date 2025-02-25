use alloy_primitives::{hex, Address, Bytes, FixedBytes, U256};
use alloy_provider::{Provider, ProviderBuilder, RootProvider};
use alloy_signer_local::PrivateKeySigner;
use alloy_transport::BoxTransport;
use gadget_anvil_testing_utils::keys::ANVIL_PRIVATE_KEYS;
use blueprint_runner::{
    config::{GadgetConfiguration, ProtocolSettings},
    error::RunnerError as Error,
    BlueprintConfig,
};
use eigensdk::{
    client_avsregistry::writer::AvsRegistryChainWriter,
    client_elcontracts::{reader::ELChainReader, writer::ELChainWriter},
    crypto_bls::BlsKeyPair,
    logging::get_test_logger,
    types::operator::Operator,
};
use tracing::info;

pub fn get_provider_http(http_endpoint: &str) -> RootProvider<BoxTransport> {
    let provider = ProviderBuilder::new()
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone()
        .boxed();

    provider
}

pub fn get_wallet_provider_http(
    http_endpoint: &str,
    signer: alloy_signer_local::PrivateKeySigner,
) -> RootProvider<BoxTransport> {
    let wallet = alloy_network::EthereumWallet::new(signer);
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(http_endpoint.parse().unwrap())
        .root()
        .clone()
        .boxed();

    provider
}

#[derive(Clone)]
pub struct Keystore {
    ecdsa_key: String,
    bls_keypair: BlsKeyPair,
}

impl Default for Keystore {
    fn default() -> Self {
        // Use first Anvil private key
        let ecdsa_key = hex::encode(ANVIL_PRIVATE_KEYS[0]);

        // Hardcoded BLS private key for testing
        let bls_private_key =
            "1371012690269088913462269866874713266643928125698382731338806296762673180359922";
        let bls_keypair = BlsKeyPair::new(bls_private_key.to_string()).expect("Invalid BLS key");

        Self {
            ecdsa_key,
            bls_keypair,
        }
    }
}

impl Keystore {
    pub fn ecdsa_private_key(&self) -> &str {
        &self.ecdsa_key
    }

    pub fn ecdsa_address(&self) -> Address {
        let private_key: PrivateKeySigner = self.ecdsa_key.parse().unwrap();
        private_key.address()
    }

    pub fn bls_keypair(&self) -> &BlsKeyPair {
        &self.bls_keypair
    }
}

#[derive(Clone)]
pub struct EigenlayerBLSConfig {
    earnings_receiver_address: Address,
    delegation_approver_address: Address,
    exit_after_register: bool,
    keystore: Keystore,
}

impl EigenlayerBLSConfig {
    /// Creates a new [`EigenlayerBLSConfig`] with the given earnings receiver and delegation approver addresses.
    ///
    /// By default, a Runner created with this config will exit after registration (Pre-Registration). To change
    /// this, use [`EigenlayerBLSConfig::with_exit_after_register`] passing false.
    pub fn new(earnings_receiver_address: Address, delegation_approver_address: Address) -> Self {
        Self {
            earnings_receiver_address,
            delegation_approver_address,
            exit_after_register: true,
            keystore: Keystore::default(),
        }
    }

    /// Sets whether the Runner should exit after registration or continue with execution.
    pub fn with_exit_after_register(mut self, should_exit_after_registration: bool) -> Self {
        self.exit_after_register = should_exit_after_registration;
        self
    }
}

#[async_trait::async_trait]
impl BlueprintConfig for EigenlayerBLSConfig {
    async fn register(&self, env: &GadgetConfiguration) -> Result<(), Error> {
        let contract_addresses = match env.protocol_settings {
            ProtocolSettings::Eigenlayer(addresses) => addresses,
            _ => return Err(Error::InvalidProtocol("Not Eigenlayer".to_string())),
        };

        // Use ANVIL private key for testing
        let operator_private_key =
            "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let provider = get_provider_http(&env.http_rpc_endpoint);

        let delegation_manager =
            eigensdk::utils::core::delegationmanager::DelegationManager::DelegationManagerInstance::new(
                contract_addresses.delegation_manager_address,
                provider,
            );
        let slasher_address = delegation_manager
            .slasher()
            .call()
            .await
            .map(|a| a._0)
            .map_err(|e| Error::Contract(e.to_string()))?;

        let logger = get_test_logger();
        let avs_registry_writer = AvsRegistryChainWriter::build_avs_registry_chain_writer(
            logger.clone(),
            env.http_rpc_endpoint.clone(),
            operator_private_key.to_string(),
            contract_addresses.registry_coordinator_address,
            contract_addresses.operator_state_retriever_address,
        )
        .await
        .map_err(|e| Error::AvsRegistry(e.to_string()))?;

        // Use local keystore for BLS key
        let operator_bls_key = self.keystore.bls_keypair();
        let digest_hash: FixedBytes<32> = FixedBytes::from([0x02; 32]);
        let now = std::time::SystemTime::now();
        let sig_expiry = now
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| U256::from(duration.as_secs()) + U256::from(86400))
            .unwrap_or_else(|_| {
                info!("System time seems to be before the UNIX epoch.");
                U256::from(0)
            });

        let quorum_nums = Bytes::from(vec![0]);

        let el_chain_reader = ELChainReader::new(
            logger,
            slasher_address,
            contract_addresses.delegation_manager_address,
            contract_addresses.rewards_coordinator_address,
            contract_addresses.avs_directory_address,
            env.http_rpc_endpoint.clone(),
        );

        let el_writer = ELChainWriter::new(
            contract_addresses.strategy_manager_address,
            contract_addresses.rewards_coordinator_address,
            el_chain_reader,
            env.http_rpc_endpoint.clone(),
            operator_private_key.to_string(),
        );

        let staker_opt_out_window_blocks = 50400u32;
        let operator_details = Operator {
            address: self.keystore.ecdsa_address(),
            earnings_receiver_address: self.earnings_receiver_address,
            delegation_approver_address: self.delegation_approver_address,
            metadata_url: Some("https://github.com/tangle-network/gadget".to_string()),
            staker_opt_out_window_blocks,
        };

        let tx_hash = el_writer
            .register_as_operator(operator_details)
            .await
            .map_err(|e| Error::Contract(e.to_string()))?;
        info!("Registered as operator for Eigenlayer {:?}", tx_hash);

        let tx_hash = avs_registry_writer
            .register_operator_in_quorum_with_avs_registry_coordinator(
                operator_bls_key.clone(),
                digest_hash,
                sig_expiry,
                quorum_nums,
                env.http_rpc_endpoint.clone(),
            )
            .await
            .map_err(|e| Error::AvsRegistry(e.to_string()))?;

        info!("Registered operator for Eigenlayer {:?}", tx_hash);
        Ok(())
    }

    async fn requires_registration(&self, env: &GadgetConfiguration) -> Result<bool, Error> {
        let contract_addresses = env
            .protocol_settings
            .eigenlayer()
            .map_err(|_| Error::InvalidProtocol("Not Eigenlayer".to_string()))?;

        // Use ANVIL private key for testing
        let operator_address = self.keystore.ecdsa_address();

        let avs_registry_reader =
            eigensdk::client_avsregistry::reader::AvsRegistryChainReader::new(
                get_test_logger(),
                contract_addresses.registry_coordinator_address,
                contract_addresses.operator_state_retriever_address,
                env.http_rpc_endpoint.clone(),
            )
            .await
            .map_err(|e| Error::AvsRegistry(e.to_string()))?;

        match avs_registry_reader
            .is_operator_registered(operator_address)
            .await
        {
            Ok(is_registered) => Ok(!is_registered),
            Err(e) => Err(Error::AvsRegistry(e.to_string())),
        }
    }

    fn should_exit_after_registration(&self) -> bool {
        self.exit_after_register
    }
}
