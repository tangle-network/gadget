use color_eyre::Result;
use gadget_sdk::config::ContextConfig;
use gadget_sdk::run::GadgetRunner;
use incredible_squaring_aggregator::{self, *};
use runner::EigenlayerGadgetRunner;
use structopt::StructOpt;
use tracing::info;

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
