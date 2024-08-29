use alloy_network::EthereumWallet;
use alloy_primitives::{Address, FixedBytes};
use alloy_provider::ProviderBuilder;
use alloy_signer::k256::ecdsa::SigningKey;
use alloy_signer_local::PrivateKeySigner;
use color_eyre::{eyre::eyre, eyre::OptionExt, Result};
use gadget_sdk::{
    env::{Protocol, StdGadgetConfiguration},
    events_watcher::{
        evm::{Config, EventWatcher},
        substrate::SubstrateEventWatcher,
        tangle::TangleEventsWatcher,
    },
    keystore::Backend,
    tangle_subxt::tangle_testnet_runtime::api::{
        self,
        runtime_types::{sp_core::ecdsa, tangle_primitives::services},
    },
    tx,
};

use incredible_squaring_blueprint::{self as blueprint, IncredibleSquaringTaskManager};

use std::sync::Arc;

#[async_trait::async_trait]
trait GadgetRunner {
    fn env(&self) -> &StdGadgetConfiguration;
    async fn register(&self) -> Result<()>;
    async fn benchmark(&self) -> Result<()>;
    async fn run(&self) -> Result<()>;
}

struct TangleGadgetRunner {
    env: StdGadgetConfiguration,
}

#[async_trait::async_trait]
impl GadgetRunner for TangleGadgetRunner {
    fn env(&self) -> &StdGadgetConfiguration {
        &self.env
    }
    async fn register(&self) -> Result<()> {
        let client = self.env.client().await?;
        let signer = self.env.first_signer()?;
        let ecdsa_signer = self.env.first_ecdsa_signer()?;
        let ecdsa_key = ecdsa_signer.public_key().to_bytes();

        let xt = api::tx().services().register(
            self.env.blueprint_id,
            services::OperatorPreferences {
                key: ecdsa::Public(ecdsa_key),
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
        let env = gadget_sdk::env::load(None)?;
        let client = env.client().await?;
        let signer = env.first_signer()?;

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

    async fn benchmark(&self) -> Result<()> {
        let env = gadget_sdk::env::load(None)?;
        let client = env.client().await?;
        let signer = env.first_signer()?;
        let xsquare_summary = blueprint::xsquare_benchmark();
        Ok(())
    }
}

struct EigenlayerGadgetRunner {
    env: StdGadgetConfiguration,
}

#[derive(Debug, Default, Clone)]
struct EigenlayerEventWatcher<C> {
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Config> EventWatcher<C> for EigenlayerEventWatcher<C> {
    const TAG: &'static str = "eigenlayer";
    type Contract =
        IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance<C::T, C::P, C::N>;
    type Event = IncredibleSquaringTaskManager::NewTaskCreated;
    const GENESIS_TX_HASH: FixedBytes<32> = FixedBytes([0; 32]);
}

#[async_trait::async_trait]
impl GadgetRunner for EigenlayerGadgetRunner {
    fn env(&self) -> &StdGadgetConfiguration {
        &self.env
    }
    async fn register(&self) -> Result<()> {
        let keystore = self.env.keystore()?;
        let ecdsa_key = keystore
            .iter_ecdsa()
            .next()
            .map(|v| v.to_sec1_bytes().to_vec().try_into())
            .transpose()
            .map_err(|_| eyre!("Failed to convert ECDSA key"))?
            .ok_or_eyre("No ECDSA key found")?;
        let signing_key: SigningKey = SigningKey::from_bytes(&ecdsa_key)?;
        let signer: PrivateKeySigner = PrivateKeySigner::from_signing_key(signing_key);

        // Implement Eigenlayer-specific registration logic here
        // For example:
        // let contract = YourEigenlayerContract::new(contract_address, provider);
        // contract.register_operator(signer).await?;

        tracing::info!("Registered operator for Eigenlayer");
        Ok(())
    }

    async fn run(&self) -> Result<()> {
        let env = gadget_sdk::env::load(Some(Protocol::Eigenlayer))?;
        let keystore = env.keystore()?;
        // get the first ECDSA key from the keystore and register with it.
        let ecdsa_key = keystore
            .iter_ecdsa()
            .next()
            .map(|v| v.to_sec1_bytes().to_vec().try_into())
            .transpose()
            .map_err(|_| eyre!("Failed to convert ECDSA key"))?
            .ok_or_eyre("No ECDSA key found")?;
        let signing_key: SigningKey = SigningKey::from_bytes(&ecdsa_key)?;
        let priv_key_signer: PrivateKeySigner = PrivateKeySigner::from_signing_key(signing_key);
        let wallet = EthereumWallet::from(priv_key_signer.clone());
        // Set up eignelayer AVS
        let contract_address = Address::from_slice(&[0; 20]);
        // Set up the HTTP provider with the `reqwest` crate.
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(env.rpc_endpoint.parse()?);
        let x_square_eigen = blueprint::XsquareEigenEventHandler {};

        let contract = IncredibleSquaringTaskManager::IncredibleSquaringTaskManagerInstance::new(
            contract_address,
            provider,
        );

        EventWatcher::run(
            &EigenlayerEventWatcher::default(),
            contract,
            // Add more handler here if we have more functions.
            vec![Box::new(x_square_eigen)],
        )
        .await?;

        Ok(())
    }

    async fn benchmark(&self) -> Result<()> {
        let env = gadget_sdk::env::load(Some(Protocol::Eigenlayer))?;
        let client = env.client().await?;
        let signer = env.first_signer()?;
        let xsquare_summery = blueprint::xsquare_benchmark();
        Ok(())
    }
}

fn create_gadget_runner(protocol: Protocol) -> Arc<dyn GadgetRunner> {
    let env = gadget_sdk::env::load(Some(protocol)).expect("Failed to load environment");
    match protocol {
        Protocol::Tangle => Arc::new(TangleGadgetRunner { env }),
        Protocol::Eigenlayer => Arc::new(EigenlayerGadgetRunner { env }),
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
    let protocol = Protocol::from_env();
    let runner = create_gadget_runner(protocol);
    // Register the operator if needed
    if runner.env().should_run_registration() {
        runner.register().await?;
    }
    // Run benchmark if needed
    if runner.env().should_run_benchmarks() {
        runner.benchmark().await?;
    }

    // Run the gadget / AVS
    runner.run().await?;

    Ok(())
}
