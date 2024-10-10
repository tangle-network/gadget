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
use gadget_sdk::run::GadgetRunner;
use gadget_sdk::{
    config::{ContextConfig, GadgetConfiguration},
    network::{
        gossip::GossipHandle,
        setup::{start_p2p_network, NetworkConfig},
    },
};
use incredible_squaring_blueprint_eigenlayer::{self, constants::*, *};
use k256::{ecdsa::SigningKey, SecretKey};
use sp_core::Pair;
use std::env;
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
        // let provider = eigensdk::utils::get_provider(&EIGENLAYER_HTTP_ENDPOINT);

        // let keystore = self.env.keystore().map_err(|e| eyre!(e))?;

        let ecdsa_subxt_key = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
        let ecdsa_secret_key_bytes = ecdsa_subxt_key.signer().seed();
        let ecdsa_secret_key =
            SecretKey::from_slice(&ecdsa_secret_key_bytes).map_err(|e| eyre!(e))?;
        let ecdsa_signing_key = SigningKey::from(&ecdsa_secret_key);
        let ecdsa_key =
            sp_core::ecdsa::Pair::from_seed_slice(&ecdsa_secret_key_bytes).map_err(|e| eyre!(e))?;

        let priv_key_signer: PrivateKeySigner =
            PrivateKeySigner::from_signing_key(ecdsa_signing_key);

        let identity = libp2p::identity::Keypair::ed25519_from_bytes(&mut SR_SECRET_BYTES.clone())
            .map_err(|e| eyre!("Unable to construct libp2p keypair: {e:?}"))?;

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

        let contract_address = Address::from_slice(&[0; 20]);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(self.env.rpc_endpoint.parse()?);

        let contract = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(
            contract_address,
            provider,
        );
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
