use color_eyre::{eyre::eyre, Result};
use gadget_sdk::config::{ContextConfig, GadgetCLICoreSettings, GadgetConfiguration, StdGadgetConfiguration};
use gadget_sdk::{
    config::Protocol,
    events_watcher::tangle::TangleEventsWatcher,
    tangle_subxt::tangle_testnet_runtime::api::{
        self,
        runtime_types::{sp_core::ecdsa, tangle_primitives::services},
    },
    tx,
    info
};
use std::io::Write;
use std::sync::Arc;
use incredible_squaring_blueprint as blueprint;
use structopt::StructOpt;
use gadget_sdk::event_listener::{EventListener, IntoTangleEventListener};
use gadget_sdk::keystore::KeystoreUriSanitizer;
use gadget_sdk::keystore::sp_core_subxt::Pair;
use gadget_sdk::run::GadgetRunner;
use gadget_sdk::tangle_subxt::subxt::tx::Signer;
use gadget_sdk::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets;
use incredible_squaring_blueprint::LocalSubstrateTestnetContext;

struct TangleGadgetRunner {
    env: GadgetConfiguration<parking_lot::RawRwLock>,
}

#[async_trait::async_trait]
impl GadgetRunner for TangleGadgetRunner {
    type Error = color_eyre::eyre::Report;

    fn config(&self) -> &StdGadgetConfiguration {
        todo!()
    }

    async fn register(&mut self) -> Result<()> {
        // TODO: Use the function in blueprint-test-utils
        if self.env.test_mode {
            info!("Skipping registration in test mode");
            return Ok(());
        }

        let client = self.env.client().await.map_err(|e| eyre!(e))?;
        let signer = self
            .env
            .first_sr25519_signer()
            .map_err(|e| eyre!(e))
            .map_err(|e| eyre!(e))?;
        let ecdsa_pair = self.env.first_ecdsa_signer().map_err(|e| eyre!(e))?;

        let xt = api::tx().services().register(
            self.env.blueprint_id,
            services::OperatorPreferences {
                key: ecdsa::Public(ecdsa_pair.signer().public().0),
                approval: services::ApprovalPrefrence::None,
                // TODO: Set the price targets
                price_targets: PriceTargets {
                    cpu: 0,
                    mem: 0,
                    storage_hdd: 0,
                    storage_ssd: 0,
                    storage_nvme: 0,
                },
            },
            Default::default(),
        );

        // send the tx to the tangle and exit.
        let result = tx::tangle::send(&client, &signer, &xt).await?;
        info!("Registered operator with hash: {:?}", result);
        Ok(())
    }

    async fn benchmark(&self) -> std::result::Result<(), Self::Error> {
        todo!()
    }

    async fn run(&self) -> Result<()> {
        let client = self.env.client().await.map_err(|e| eyre!(e))?;
        let signer = self.env.first_sr25519_signer().map_err(|e| eyre!(e))?;

        info!("Starting the event watcher for {} ...", signer.account_id());

        let x_square = blueprint::XsquareEventHandler {
            service_id: self.env.service_id.unwrap(),
            env: self.env.clone(),
            signer,
        };

        let program = TangleEventsWatcher {
            span: self.env.span.clone(),
            client,
            handlers: vec![Arc::new(x_square)],
        };

        // TODO: maybe self.run() should return this program instead of running it
        // here

        Ok(())
    }
}

async fn create_gadget_runner(
    config: ContextConfig,
) -> (
    GadgetConfiguration<parking_lot::RawRwLock>,
    Box<dyn GadgetRunner<Error = color_eyre::Report>>,
) {
    let env = gadget_sdk::config::load(config).expect("Failed to load environment");
    match env.protocol {
        Protocol::Tangle => (env.clone(), Box::new(TangleGadgetRunner { env })),
        /*
        Protocol::Eigenlayer => (
            env.clone(),
            Box::new(blueprint::eigenlayer::EigenlayerGadgetRunner::new(env).await),
        ),*/
        _ => panic!("Unsupported protocol Eigenlayer. Gadget/Tangle need U256 support."),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    gadget_sdk::logging::setup_log();
    // Load the environment and create the gadget runner
    let config = ContextConfig::from_args();

    let (env, mut runner) = create_gadget_runner(config.clone()).await;

    info!("~~~ Executing the incredible squaring blueprint ~~~");

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
    _env: &GadgetConfiguration<parking_lot::RawRwLock>,
    config: &ContextConfig,
) -> Result<()> {
    // create a file to denote we have started
    if let GadgetCLICoreSettings::Run {
        keystore_uri: base_path,
        test_mode,
        ..
    } = &config.gadget_core_settings
    {
        if !*test_mode {
            return Ok(());
        }
        let path = base_path.sanitize_file_path().join("test_started.tmp");
        let mut file = std::fs::File::create(&path)?;
        file.write_all(b"test_started")?;
        info!("Successfully wrote test file to {}", path.display())
    }

    Ok(())
}
