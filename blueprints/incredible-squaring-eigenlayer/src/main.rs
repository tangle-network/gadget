use alloy_network::EthereumWallet;
use alloy_primitives::{address, Address, Bytes, FixedBytes, U256};
use alloy_provider::ProviderBuilder;
use alloy_signer_local::PrivateKeySigner;
use color_eyre::{eyre::eyre, Result};
use eigensdk::client_avsregistry::writer::AvsRegistryChainWriter;
use eigensdk::client_elcontracts::reader::ELChainReader;
use eigensdk::client_elcontracts::writer::ELChainWriter;
use eigensdk::crypto_bls::BlsKeyPair;
use eigensdk::logging::get_test_logger;
use eigensdk::types::operator::Operator;
use gadget_sdk::events_watcher::InitializableEventHandler;
use gadget_sdk::info;
use gadget_sdk::keystore::backend::fs::FilesystemKeystore;
use gadget_sdk::keystore::backend::GenericKeyStore;
use gadget_sdk::keystore::Backend;
use gadget_sdk::keystore::BackendExt;
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
use std::env;
use std::path::Path;
use std::path::PathBuf;
use structopt::lazy_static::lazy_static;
use structopt::StructOpt;
use uuid::Uuid;

lazy_static! {
    /// 1 day
    static ref SIGNATURE_EXPIRY: U256 = U256::from(86400);
}

// Environment variables with default values
lazy_static! {
    static ref EIGENLAYER_HTTP_ENDPOINT: String = env::var("EIGENLAYER_HTTP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:8545".to_string());
    static ref EIGENLAYER_WS_ENDPOINT: String =
        env::var("EIGENLAYER_WS_ENDPOINT").unwrap_or_else(|_| "ws://localhost:8546".to_string());
    static ref PRIVATE_KEY: String = env::var("PRIVATE_KEY").unwrap_or_else(|_| {
        "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string()
    });
    static ref SR_SECRET_BYTES: Vec<u8> = env::var("SR_SECRET_BYTES")
        .map(|v| v.into_bytes())
        .unwrap_or_else(|_| vec![0; 32]);
    static ref REGISTRY_COORDINATOR_ADDRESS: Address =
        address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690");
    static ref OPERATOR_STATE_RETRIEVER_ADDRESS: Address =
        address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8");
    static ref DELEGATION_MANAGER_ADDRESS: Address =
        address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9");
    static ref STRATEGY_MANAGER_ADDRESS: Address =
        address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707");
    static ref AVS_DIRECTORY_ADDRESS: Address =
        address!("0000000000000000000000000000000000000000");
    static ref OPERATOR_ADDRESS: Address = address!("f39fd6e51aad88f6f4ce6ab8827279cfffb92267");
    static ref OPERATOR_METADATA_URL: String =
        "https://github.com/tangle-network/eigensdk-rs/blob/main/test-utils/metadata.json"
            .to_string();
    static ref MNEMONIC_SEED: String = env::var("MNEMONIC_SEED").unwrap_or_else(|_| {
        "test test test test test test test test test test test junk".to_string()
    });
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

        let provider = eigensdk::utils::get_provider(&EIGENLAYER_HTTP_ENDPOINT);

        let delegation_manager = eigensdk::utils::binding::DelegationManager::new(
            *DELEGATION_MANAGER_ADDRESS,
            provider.clone(),
        );
        let slasher_address = delegation_manager.slasher().call().await.map(|a| a._0)?;

        let test_logger = get_test_logger();
        let avs_registry_writer = AvsRegistryChainWriter::build_avs_registry_chain_writer(
            test_logger.clone(),
            EIGENLAYER_HTTP_ENDPOINT.to_string(),
            PRIVATE_KEY.to_string(),
            *REGISTRY_COORDINATOR_ADDRESS,
            *OPERATOR_STATE_RETRIEVER_ADDRESS,
        )
        .await
        .expect("avs writer build fail ");

        // TODO: Retrieve BLS Secret Key from Keystore
        let bls_key_pair = BlsKeyPair::new(
            "1371012690269088913462269866874713266643928125698382731338806296762673180359922"
                .to_string(),
        )
        .map_err(|e| eyre!(e))?;

        let digest_hash: FixedBytes<32> = FixedBytes::from([0x02; 32]);

        let now = std::time::SystemTime::now();
        let sig_expiry = now
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| U256::from(duration.as_secs()) + *SIGNATURE_EXPIRY)
            .unwrap_or_else(|_| {
                info!("System time seems to be before the UNIX epoch.");
                U256::from(0)
            });

        let quorum_nums = Bytes::from(vec![0]);

        let el_chain_reader = ELChainReader::new(
            get_test_logger().clone(),
            slasher_address,
            *DELEGATION_MANAGER_ADDRESS,
            *AVS_DIRECTORY_ADDRESS,
            EIGENLAYER_HTTP_ENDPOINT.to_string(),
        );

        let el_writer = ELChainWriter::new(
            *DELEGATION_MANAGER_ADDRESS,
            *STRATEGY_MANAGER_ADDRESS,
            el_chain_reader,
            EIGENLAYER_HTTP_ENDPOINT.to_string(),
            PRIVATE_KEY.to_string(),
        );

        let operator_details = Operator {
            address: *OPERATOR_ADDRESS,
            earnings_receiver_address: *OPERATOR_ADDRESS,
            delegation_approver_address: *OPERATOR_ADDRESS,
            staker_opt_out_window_blocks: 50400u32,
            metadata_url: Some(OPERATOR_METADATA_URL.clone()),
        };

        el_writer
            .register_as_operator(operator_details)
            .await
            .map_err(|e| eyre!(e))?;

        avs_registry_writer
            .register_operator_in_quorum_with_avs_registry_coordinator(
                bls_key_pair,
                digest_hash,
                sig_expiry,
                quorum_nums,
                EIGENLAYER_HTTP_ENDPOINT.to_string(),
            )
            .await?;

        info!("Registered operator for Eigenlayer");
        Ok(())
    }

    async fn benchmark(&self) -> std::result::Result<(), Self::Error> {
        todo!()
    }

    async fn run(&mut self) -> Result<()> {
        // Get the ECDSA key from the private key seed using alloy
        let signer: PrivateKeySigner = PRIVATE_KEY.parse().expect("failed to generate wallet ");
        let wallet = EthereumWallet::from(signer);

        let identity = libp2p::identity::Keypair::ed25519_from_bytes(&mut SR_SECRET_BYTES.clone())
            .map_err(|e| eyre!("Unable to construct libp2p keypair: {e:?}"))?;

        let contract_address = Address::from_slice(&[0; 20]);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(self.env.rpc_endpoint.parse()?);

        let contract = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(
            contract_address,
            provider,
        );

        info!("Starting the Incredible Squaring event handler");
        let x_square_eigen = XsquareEigenEventHandler::<NodeConfig> {
            contract: contract.into(),
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
#[allow(clippy::needless_return)]
async fn main() -> Result<()> {
    gadget_sdk::logging::setup_log();
    let config = ContextConfig::from_args();
    let env = gadget_sdk::config::load(config).expect("Failed to load environment");
    let mut runner = Box::new(EigenlayerGadgetRunner::new(env.clone()).await);

    info!("~~~ Executing the incredible squaring blueprint ~~~");

    info!("Registering...");
    if env.should_run_registration() {
        runner.register().await?;
    }

    info!("Running...");
    runner.run().await?;

    info!("Exiting...");
    Ok(())
}
