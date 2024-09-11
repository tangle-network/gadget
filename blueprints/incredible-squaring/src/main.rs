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
use std::io::Write;

use incredible_squaring_blueprint as blueprint;

use gadget_sdk::env::{ContextConfig, GadgetConfiguration};
use std::sync::Arc;
use structopt::StructOpt;

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
        self.env
            .logger
            .info(format!("Registered operator with hash: {:?}", result));
        Ok(())
    }

    async fn run(&self) -> Result<()> {
        /*        let config = ContextConfig {
            gadget_core_settings: GadgetCLICoreSettings::Run {
                bind_addr: self.env.bind_addr,
                bind_port: self.env.bind_port,
                test_mode: self.env.test_mode,
                logger: self.env.logger.clone(),
                url: self.env.ur,
                bootnodes: None,
                base_path: Default::default(),
                chain: Default::default(),
                verbose: 0,
                pretty: false,
                keystore_password: None,
            },
        };*/

        //let env = gadget_sdk::env::load(None, config).map_err(|e| eyre!(e))?;

        let client = self.env.client().await.map_err(|e| eyre!(e))?;
        let signer = self.env.first_signer().map_err(|e| eyre!(e))?;
        let logger = self.env.logger.clone();

        let x_square = blueprint::XsquareEventHandler {
            service_id: self.env.service_id.unwrap(),
            signer,
            logger,
        };

        self.env.logger.info("Starting the event watcher ...");

        SubstrateEventWatcher::run(
            &TangleEventsWatcher {
                logger: self.env.logger.clone(),
            },
            client,
            // Add more handler here if we have more functions.
            vec![Box::new(x_square)],
        )
        .await?;

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
    // color_eyre::install()?;
    gadget_sdk::setup_log();

    // Load the environment and create the gadget runner
    // TODO: Place protocol in the config
    let protocol = Protocol::Tangle;
    let config = ContextConfig::from_args();

    let (env, runner) = create_gadget_runner(protocol, config.clone());

    env.logger
        .info("~~~ Executing the incredible squaring blueprint ~~~");

    check_for_test(&env, &config)?;

    // Register the operator if needed
    if env.should_run_registration() {
        runner.register().await?;
    }

    // Run the gadget / AVS
    runner.run().await?;

    Ok(())
}

#[allow(irrefutable_let_patterns)]
fn check_for_test(
    env: &GadgetConfiguration<parking_lot::RawRwLock>,
    config: &ContextConfig,
) -> Result<()> {
    // create a file to denote we have started
    if let gadget_sdk::env::GadgetCLICoreSettings::Run {
        base_path,
        test_mode,
        ..
    } = &config.gadget_core_settings
    {
        if !*test_mode {
            return Ok(());
        }
        let path = base_path.join("test_started.tmp");
        let mut file = std::fs::File::create(&path)?;
        file.write_all(b"test_started")?;
        env.logger.info(format!(
            "Successfully wrote test file to {}",
            path.display()
        ))
    }

    Ok(())
}
