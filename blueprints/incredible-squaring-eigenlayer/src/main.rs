use alloy_network::EthereumWallet;
use alloy_primitives::{address, Address, Bytes, FixedBytes, U256};
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use color_eyre::{
    eyre::{eyre, OptionExt},
    Result,
};
use eigensdk::client_avsregistry::writer::AvsRegistryChainWriter;
use eigensdk::client_elcontracts::reader::ELChainReader;
use eigensdk::client_elcontracts::writer::ELChainWriter;
use eigensdk::crypto_bls::BlsKeyPair;
use eigensdk::logging::get_test_logger;
use eigensdk::types::operator::Operator;
use gadget_sdk::events_watcher::InitializableEventHandler;
use gadget_sdk::info;
use gadget_sdk::keystore::Backend;
use gadget_sdk::run::GadgetRunner;
use gadget_sdk::{
    config::{ContextConfig, GadgetConfiguration},
    network::{
        gossip::GossipHandle,
        setup::{start_p2p_network, NetworkConfig},
    },
};
use incredible_squaring_blueprint_eigenlayer::{self, *};
use k256::{ecdsa::SigningKey, SecretKey};
use sp_core::Pair;
use structopt::lazy_static::lazy_static;
use structopt::StructOpt;

lazy_static! {
    /// 1 day
    static ref SIGNATURE_EXPIRY: U256 = U256::from(86400);
}
pub struct EigenlayerGadgetRunner<R: lock_api::RawRwLock> {
    pub env: GadgetConfiguration<R>,
}

impl<R: lock_api::RawRwLock> EigenlayerGadgetRunner<R> {
    pub async fn new(env: GadgetConfiguration<R>) -> Self {
        Self { env }
    }
}

#[async_trait::async_trait]
impl GadgetRunner for EigenlayerGadgetRunner<parking_lot::RawRwLock> {
    type Error = color_eyre::eyre::Report;

    fn config(&self) -> &GadgetConfiguration<parking_lot::RawRwLock> {
        todo!()
    }

    async fn register(&mut self) -> Result<()> {
        if self.env.test_mode {
            info!("Skipping registration in test mode");
            return Ok(());
        }

        // let http_endpoint = "http://127.0.0.1:8545";
        let http_endpoint = std::env::var("EIGENLAYER_HTTP_ENDPOINT")
            .expect("EIGENLAYER_HTTP_ENDPOINT must be set");
        let _ws_endpoint =
            std::env::var("EIGENLAYER_WS_ENDPOINT").expect("EIGENLAYER_WS_ENDPOINT must be set");

        let pvt_key = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

        // TODO: Should be pulled from environment variables
        // let service_manager_address = address!("67d269191c92caf3cd7723f116c85e6e9bf55933");
        let registry_coordinator_address = address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
        let operator_state_retriever_address = address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
        let delegation_manager_address = address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
        let strategy_manager_address = address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");
        // let erc20_mock_address = address!("7969c5ed335650692bc04293b07f5bf2e7a673c0");
        let avs_directory_address = address!("0000000000000000000000000000000000000000");

        let provider = eigensdk::utils::get_provider(&http_endpoint);

        // TODO: Move Slasher retrieval into SDK
        let delegation_manager = eigensdk::utils::binding::DelegationManager::new(
            delegation_manager_address,
            provider.clone(),
        );
        let slasher_address = delegation_manager.slasher().call().await.map(|a| a._0)?;

        let test_logger = get_test_logger();
        let avs_registry_writer = AvsRegistryChainWriter::build_avs_registry_chain_writer(
            test_logger.clone(),
            http_endpoint.to_string(),
            pvt_key.to_string(),
            registry_coordinator_address,
            operator_state_retriever_address,
        )
        .await
        .expect("avs writer build fail ");

        // TODO: Retrieve BLS Secret Key from Keystore
        // Create a new key pair instance using the secret key
        let bls_key_pair = BlsKeyPair::new(
            "1371012690269088913462269866874713266643928125698382731338806296762673180359922"
                .to_string(),
        )
        .map_err(|e| eyre!(e))?;

        let digest_hash: FixedBytes<32> = FixedBytes::from([0x02; 32]);

        // Get the current SystemTime
        let now = std::time::SystemTime::now();
        let mut sig_expiry: U256 = U256::from(0);
        // Convert SystemTime to a Duration since the UNIX epoch
        if let Ok(duration_since_epoch) = now.duration_since(std::time::UNIX_EPOCH) {
            // Convert the duration to seconds
            let seconds = duration_since_epoch.as_secs(); // Returns a u64

            // Convert seconds to U256
            sig_expiry = U256::from(seconds) + *SIGNATURE_EXPIRY;
        } else {
            info!("System time seems to be before the UNIX epoch.");
        }
        let quorum_nums = Bytes::from(vec![0]);

        // A new ElChainReader instance
        let el_chain_reader = ELChainReader::new(
            get_test_logger().clone(),
            slasher_address,
            delegation_manager_address,
            avs_directory_address,
            http_endpoint.to_string(),
        );
        // A new ElChainWriter instance
        let el_writer = ELChainWriter::new(
            delegation_manager_address,
            strategy_manager_address,
            el_chain_reader,
            http_endpoint.to_string(),
            pvt_key.to_string(),
        );

        let operator_addr = address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92266");
        let operator_details = Operator {
            address: operator_addr,
            earnings_receiver_address: operator_addr,
            delegation_approver_address: operator_addr,
            staker_opt_out_window_blocks: 50400u32,
            metadata_url: Some(
                "https://github.com/tangle-network/eigensdk-rs/blob/main/test-utils/metadata.json"
                    .to_string(),
            ), // TODO: Metadata should be from Environment Variable
        };

        // Register the address as operator in delegation manager
        el_writer
            .register_as_operator(operator_details)
            .await
            .map_err(|e| eyre!(e))?;

        // Register the operator in registry coordinator
        avs_registry_writer
            .register_operator_in_quorum_with_avs_registry_coordinator(
                bls_key_pair,
                digest_hash,
                sig_expiry,
                quorum_nums,
                http_endpoint.to_string(),
            )
            .await?;

        info!("Registered operator for Eigenlayer");
        Ok(())
    }

    async fn benchmark(&self) -> std::result::Result<(), Self::Error> {
        todo!()
    }

    async fn run(&mut self) -> Result<()> {
        let http_endpoint = std::env::var("EIGENLAYER_HTTP_ENDPOINT")
            .expect("EIGENLAYER_HTTP_ENDPOINT must be set");
        let _ws_endpoint =
            std::env::var("EIGENLAYER_WS_ENDPOINT").expect("EIGENLAYER_WS_ENDPOINT must be set");
        let provider = eigensdk::utils::get_provider(&http_endpoint);

        let keystore = self.env.keystore().map_err(|e| eyre!(e))?;

        // TODO: greatly simplify all this logic by using the GenericKeyStore interface
        // ED25519 Key Retrieval
        let ed_key = keystore
            .iter_ed25519()
            .next()
            .ok_or_eyre("Unable to find ED25519 key")?;
        let ed_public_bytes = ed_key.as_ref(); // 32 byte len
        let ed_public = ed25519_zebra::VerificationKey::try_from(ed_public_bytes)
            .map_err(|_| eyre!("Unable to create ed25519 public key"))?;

        // ECDSA Key Retrieval
        let ecdsa_subxt_key = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
        let ecdsa_secret_key_bytes = ecdsa_subxt_key.signer().seed();
        let ecdsa_secret_key =
            SecretKey::from_slice(&ecdsa_secret_key_bytes).map_err(|e| eyre!(e))?;
        let ecdsa_signing_key = SigningKey::from(&ecdsa_secret_key);
        let ecdsa_key =
            sp_core::ecdsa::Pair::from_seed_slice(&ecdsa_secret_key_bytes).map_err(|e| eyre!(e))?;

        // Construct Signer
        let priv_key_signer: PrivateKeySigner =
            PrivateKeySigner::from_signing_key(ecdsa_signing_key);

        let sr_secret = keystore
            .expose_ed25519_secret(&ed_public)
            .map_err(|e| eyre!(e))?
            .ok_or_eyre("Unable to find ED25519 secret")?;
        let mut sr_secret_bytes = sr_secret.as_ref().to_vec(); // 64 byte len

        let identity = libp2p::identity::Keypair::ed25519_from_bytes(&mut sr_secret_bytes)
            .map_err(|e| eyre!("Unable to construct libp2p keypair: {e:?}"))?;

        // TODO: Fill in and find the correct values for the network configuration
        // TODO: Implementations for reading set of operators from Tangle & Eigenlayer
        let network_config: NetworkConfig = NetworkConfig {
            identity,
            ecdsa_key,
            bootnodes: vec![],
            bind_ip: self.env.bind_addr,
            bind_port: self.env.bind_port,
            topics: vec!["__TESTING_INCREDIBLE_SQUARING".to_string()],
        };

        let _network: GossipHandle =
            start_p2p_network(network_config).map_err(|e| eyre!(e.to_string()))?;

        let wallet = EthereumWallet::from(priv_key_signer.clone());

        // Set up eigenlayer AVS
        let contract_address = Address::from_slice(&[0; 20]);
        // Set up the HTTP provider with the `reqwest` crate.
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(self.env.rpc_endpoint.parse()?);

        let contract = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(
            contract_address,
            provider,
        );
        // TODO: Make sure to pass T: Config to the XSquaredEigenEventHandler
        let x_square_eigen = XsquareEigenEventHandler::<NodeConfig> {
            // ctx: blueprint::MyContext { network, keystore },
            contract: IncredibleSquaringTaskManagerInstanceWrapper::from(contract), // call .into() to convert to IncredibleSquaringTaskManagerInstanceWrapper
        };
        let finished = x_square_eigen
            .init_event_handler()
            .await
            .expect("Event Listener init already called");
        let res = finished.await;
        info!("Event handler finished with {res:?}");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    gadget_sdk::logging::setup_log();
    // Load the environment and create the gadget runner
    let config = ContextConfig::from_args();
    let env = gadget_sdk::config::load(config).expect("Failed to load environment");
    let mut runner = Box::new(EigenlayerGadgetRunner::new(env.clone()).await);

    info!("~~~ Executing the incredible squaring blueprint ~~~");

    info!("Registering...");
    // Register the operator if needed
    if env.should_run_registration() {
        runner.register().await?;
    }

    info!("Running...");
    // Run the gadget / AVS
    runner.run().await?;

    info!("Exiting...");
    Ok(())
}
