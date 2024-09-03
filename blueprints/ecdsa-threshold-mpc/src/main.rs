use gadget_sdk as sdk;
use std::convert::Infallible;
use subxt::PolkadotConfig;

use color_eyre::{eyre::OptionExt, Result};
use sdk::{
    events_watcher::tangle::TangleEventsWatcher, keystore::backend::GenericKeyStore,
    keystore::Backend, tangle_subxt::*,
};

pub mod jobs;
pub mod mpc;

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
    let env = sdk::config::load(None)?;
    let keystore = env.keystore()?;
    let signer = env.first_signer()?;
    let client: subxt::OnlineClient<PolkadotConfig> =
        subxt::OnlineClient::from_url(&env.rpc_endpoint).await?;

    // // Create the event handler from the job
    // let keygen_job = KeygenEventHandler {
    //     service_id: env.service_id,
    //     signer,
    // };

    // let signing_job = SigningEventHandler {
    //     service_id: env.service_id,
    //     signer,
    // };

    // let key_refresh_job = KeyRefreshEventHandler {
    //     service_id: env.service_id,
    //     signer,
    // };

    // tracing::info!("Starting the event watcher ...");

    // SubstrateEventWatcher::run(
    //     &TangleEventsWatcher,
    //     client,
    //     // Add more handler here if we have more functions.
    //     vec![
    //         Box::new(keygen_job),
    //         Box::new(signing_job),
    //         Box::new(key_refresh_job),
    //     ],
    // )
    // .await?;
    Ok(())
}
