use color_eyre::{eyre::OptionExt, Result};
use gadget_sdk::{
    events_watcher::tangle::TangleEventsWatcher, events_watcher::SubstrateEventWatcher,
    keystore::Backend, tangle_subxt::subxt,
};

use incredible_squaring_blueprint as blueprint;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let env_filter = tracing_subscriber::EnvFilter::from_default_env();
    let logger = tracing_subscriber::fmt()
        .compact()
        .with_target(true)
        .with_env_filter(env_filter);
    logger.init();

    let env = gadget_sdk::env::load()?;
    let keystore = env.keystore()?;
    let client = subxt::OnlineClient::from_url(&env.tangle_rpc_endpoint).await?;

    let sr25519_pubkey = keystore
        .iter_sr25519()
        .next()
        .ok_or_eyre("No sr25519 keys found in the keystore")?;
    let sr25519_secret = keystore
        .expose_sr25519_secret(&sr25519_pubkey)?
        .ok_or_eyre("No sr25519 secret found in the keystore")?;

    let mut seed = [0u8; 32];
    seed.copy_from_slice(&sr25519_secret.to_bytes()[0..32]);
    let signer = subxt_signer::sr25519::Keypair::from_secret_key(seed)?;

    let x_square = blueprint::XsquareEventHandler {
        service_id: env.service_id,
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
