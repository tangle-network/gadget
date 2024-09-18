use color_eyre::{eyre::eyre, Result};
use gadget_sdk::{
    env::Protocol,
    events_watcher::{substrate::SubstrateEventWatcher, tangle::TangleEventsWatcher},
    tangle_subxt::tangle_testnet_runtime::api::{
        self,
        runtime_types::{sp_core::ecdsa, tangle_primitives::services},
    },
    tx,
};
use incredible_squaring_blueprint as blueprint;
use gadget_sdk::env::{ContextConfig, GadgetConfiguration};
use std::sync::Arc;
use structopt::StructOpt;

pub mod runner;

#[async_trait::async_trait]
trait GadgetRunner {
    async fn register(&self) -> Result<()>;
    async fn run(&self) -> Result<()>;
}

struct TangleGadgetRunner {
    env: gadget_sdk::env::GadgetConfiguration<parking_lot::RawRwLock>,
}

#[async_trait::async_trait]
impl GadgetRunner for TangleGadgetRunner {
    async fn register(&self) -> Result<()> {
        // TODO: Use the function in blueprint-test-utils
        if self.env.test_mode {
            self.env.logger.info("Skipping registration in test mode");
            return Ok(());
        }

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
        let config = ContextConfig {
            bind_ip: self.env.bind_addr,
            bind_port: self.env.bind_port,
            test_mode: self.env.test_mode,
            logger: self.env.logger.clone(),
        };

        let env = gadget_sdk::env::load(None, config).map_err(|e| eyre!(e))?;
        let client = env.client().await.map_err(|e| eyre!(e))?;
        let signer = env.first_signer().map_err(|e| eyre!(e))?;

        let tangle_avs = blueprint::TangleAvsEventHandler {
            service_id: env.service_id.unwrap(),
            signer,
        };

        tracing::info!("Starting the event watcher ...");

        SubstrateEventWatcher::run(
            &TangleEventsWatcher,
            client,
            // TODO: we want to handle slashing across EigenLayer and Tangle - where do we want this to happen?
            vec![Box::new(tangle_avs)],
        )
        .await?;

        // TODO: This is currently blocking - we want to gracefully start and manage tasks here
        runner::run_tangle_validator().await?;

        Ok(())
    }
}

fn create_gadget_runner(
    protocol: Protocol,
    config: ContextConfig,
) -> (
    GadgetConfiguration<parking_lot::RawRwLock>,
    Arc<dyn GadgetRunner>,
) {
    let env = gadget_sdk::env::load(Some(protocol), config).expect("Failed to load environment");
    match protocol {
        Protocol::Tangle => (env.clone(), Arc::new(TangleGadgetRunner { env })),
        Protocol::Eigenlayer => panic!("Eigenlayer not implemented yet"),
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
    // TODO: Place protocol in the config
    let protocol = Protocol::Tangle;
    let config = ContextConfig::from_args();
    let (env, runner) = create_gadget_runner(protocol, config);
    // Register the operator if needed
    if env.should_run_registration() {
        runner.register().await?;
    }

    // Run the gadget / AVS
    runner.run().await?;

    Ok(())
}
