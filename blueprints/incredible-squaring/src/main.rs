use std::net::IpAddr;
use alloy_network::EthereumWallet;
use alloy_primitives::{Address, FixedBytes};
use alloy_provider::ProviderBuilder;
use alloy_signer::k256::ecdsa::SigningKey;
use alloy_signer_local::PrivateKeySigner;
use color_eyre::{eyre::eyre, eyre::OptionExt, Result};
use gadget_sdk::{
    env::Protocol,
    events_watcher::{
        evm::{Config, EventWatcher},
        substrate::SubstrateEventWatcher,
        tangle::TangleEventsWatcher,
    },
    keystore::Backend,
    network::{
        gossip::NetworkService,
        setup::{start_p2p_network, NetworkConfig},
    },
    tangle_subxt::tangle_testnet_runtime::api::{
        self,
        runtime_types::{sp_core::ecdsa, tangle_primitives::services},
    },
    tx,
};

use incredible_squaring_blueprint::{self as blueprint, IncredibleSquaringTaskManager};

use gadget_sdk::events_watcher::tangle::TangleConfig;
use gadget_sdk::tangle_subxt::subxt;
use lock_api::{GuardSend, RwLock};
use std::sync::Arc;
use gadget_common::config::DebugLogger;
use gadget_sdk::env::GadgetConfiguration;
use gadget_sdk::network::gossip::GossipHandle;

#[async_trait::async_trait]
trait GadgetRunner {
    async fn register(&self) -> Result<()>;
    async fn run(&self) -> Result<()>;
}

struct TangleGadgetRunner<R: lock_api::RawRwLock> {
    env: gadget_sdk::env::GadgetConfiguration<R>,
}

#[async_trait::async_trait]
impl GadgetRunner for TangleGadgetRunner<parking_lot::RawRwLock> {
    async fn register(&self) -> Result<()> {
        let client = self.env.client().await.map_err(|e| eyre!(e))?;
        let signer = self.env.first_signer().map_err(|e| eyre!(e))?;
        let ecdsa_pair = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;

        let xt = api::tx().services().register(
            self.env.blueprint_id,
            services::OperatorPreferences {
                key: ecdsa::Public(ecdsa_pair.public_key().0),
                approval: services::ApprovalPrefrence::None,
            },
            Default::default(),
        );
        // send the tx to the tangle and exit.
        let result = tx::tangle::send(&client, &signer, &xt).await?;
        tracing::info!("Registered operator with hash: {:?}", result);
        Ok(())
    }

    async fn run(&self) -> Result<()> {
        let env = gadget_sdk::env::load(None).map_err(|e| eyre!(e))?;
        let client = env.client().await.map_err(|e| eyre!(e))?;
        let signer = env.first_signer().map_err(|e| eyre!(e))?;

        let x_square = blueprint::XsquareEventHandler {
            service_id: env.service_id.unwrap(),
            signer,
        };

        tracing::info!("Starting the event watcher ...");

        SubstrateEventWatcher::run(
            &TangleEventsWatcher,
            client,
            // Add more handler here if we have more functions.
            vec![Box::new(x_square)],
        )
        .await?;

        Ok(())
    }
}

struct EigenlayerGadgetRunner<R: lock_api::RawRwLock> {
    env: GadgetConfiguration<R>,
}

struct EigenlayerEventWatcher<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Config> EventWatcher<T> for EigenlayerEventWatcher<T> {
    const TAG: &'static str = "eigenlayer";
    type Contract =
        IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance<T::T, T::P, T::N>;
    type Event = IncredibleSquaringTaskManager::NewTaskCreated;
    const GENESIS_TX_HASH: FixedBytes<32> = FixedBytes::from_slice(&[0; 32]);
}

#[async_trait::async_trait]
impl GadgetRunner for EigenlayerGadgetRunner<parking_lot::RawRwLock> {
    async fn register(&self) -> Result<()> {
        let keystore = self.env.keystore().map_err(|e| eyre!(e))?;
        let ecdsa_key = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
        let signing_key = SigningKey::from(ecdsa_key.public_key());
        let signer: PrivateKeySigner = PrivateKeySigner::from_signing_key(signing_key);

        // Implement Eigenlayer-specific registration logic here
        // For example:
        // let contract = YourEigenlayerContract::new(contract_address, provider);
        // contract.register_operator(signer).await?;

        tracing::info!("Registered operator for Eigenlayer");
        Ok(())
    }

    async fn run(&self) -> Result<()> {
        let env = gadget_sdk::env::load(Some(Protocol::Eigenlayer)).map_err(|e| eyre!(e))?;
        let keystore = env.keystore().map_err(|e| eyre!(e))?;
        // get the first ECDSA key from the keystore and register with it.
        let ecdsa_key = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;
        let signing_key = SigningKey::from(ecdsa_key.public_key());
        let priv_key_signer: PrivateKeySigner = PrivateKeySigner::from_signing_key(signing_key);
        let wallet = EthereumWallet::from(priv_key_signer.clone());
        // Set up eignelayer AVS
        let contract_address = Address::from_slice(&[0; 20]);
        // Set up the HTTP provider with the `reqwest` crate.
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(env.rpc_endpoint.parse()?);



        // TODO: Fill in and find the correct values for the network configuration
        // TODO: Implementations for reading set of operators from Tangle & Eigenlayer
        let network_config: NetworkConfig = NetworkConfig {
            identity: self.env.first_signer().map_err(|e| eyre!(e))?,
            role_key: self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?, // TODO: ROLE KEY IS DEPRECATED
            bootnodes: vec![],
            bind_ip: IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            bind_port: 9999u16,
            topics: vec!["PLACEHOLDER".to_string()],
            logger: DebugLogger::default(),
        };
        let network: GossipHandle = start_p2p_network(network_config).map_err(|e| eyre!(e))?;
        // let x_square_eigen = blueprint::XsquareEigenEventHandler {
        //     ctx: blueprint::MyContext { network, keystore },
        // };
        //
        // let contract: IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(contract_address, provider);
        //
        // EventWatcher::run(
        //     &EigenlayerEventWatcher,
        //     contract,
        //     // Add more handler here if we have more functions.
        //     vec![Box::new(x_square_eigen)],
        // )
        // .await?;

        Ok(())
    }
}

fn create_gadget_runner(protocol: Protocol) -> (GadgetConfiguration<parking_lot::RawRwLock>, Arc<dyn GadgetRunner>) {
    let env = gadget_sdk::env::load(Some(protocol)).expect("Failed to load environment");
    match protocol {
        Protocol::Tangle => (env.clone(), Arc::new(TangleGadgetRunner { env })),
        Protocol::Eigenlayer => (env.clone(), Arc::new(EigenlayerGadgetRunner { env })),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let env_filter = tracing_subscriber::EnvFilter::from_default_env();
    let logger = tracing_subscriber::fmt()
        .compact()
        .with_target(true)
        .with_env_filter(env_filter);
    logger.init();

    // Load the environment and create the gadget runner
    let protocol = Protocol::from_env().unwrap_or(Protocol::Tangle);
    let (env, runner) = create_gadget_runner(protocol);
    // Register the operator if needed
    if env.should_run_registration() {
        runner.register().await?;
    }

    // Run the gadget / AVS
    runner.run().await?;

    Ok(())
}
