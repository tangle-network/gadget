use gadget_sdk as sdk;
// use std::convert::Infallible;

use color_eyre::Result;
// use sdk::{
// events_watcher::tangle::TangleEventsWatcher, keystore::backend::GenericKeyStore,
// keystore::Backend, tangle_subxt::*,
// };

pub mod runner;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Initialize the logger
    let env_filter = tracing_subscriber::EnvFilter::from_default_env();
    tracing_subscriber::fmt()
        .compact()
        .with_target(true)
        .with_env_filter(env_filter)
        .init();

    // Initialize the environment
    // TODO: Error Handling and uncomment/fix Signer & Client
    let env = sdk::env::load(None).unwrap(); //?;
    let _keystore = env.keystore().unwrap(); //?;

    // let signer = keystore.first_signer()?;
    // let client = subxt::OnlineClient::from_url(&env.tangle_rpc_endpoint).await?;

    Ok(())
}
