// use eigensdk_rs::;
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let env_filter = tracing_subscriber::EnvFilter::from_default_env();
    let logger = tracing_subscriber::fmt()
        .compact()
        .with_target(true)
        .with_env_filter(env_filter);
    logger.init();

    // TODO: Setup the Aggregator | eigensdk-rs

    // TODO: Run the Aggregator | eigensdk-rs

    Ok(())
}
