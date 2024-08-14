use color_eyre::{eyre::eyre, eyre::OptionExt, Result};
use gadget_sdk::{
    events_watcher::{tangle::TangleEventsWatcher, SubstrateEventWatcher},
    keystore::Backend,
    tangle_subxt::tangle_testnet_runtime::api::{
        self, runtime_types::sp_core::ecdsa, runtime_types::tangle_primitives::services,
    },
    tx,
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
    let client = env.client().await?;
    let signer = env.first_signer()?;
    if env.should_run_registration() {
        // We should register our operator on the blueprint.
        let keystore = env.keystore()?;
        // get the first ECDSA key from the keystore and register with it.
        let ecdsa_key = keystore
            .iter_ecdsa()
            .next()
            .map(|v| v.to_sec1_bytes().to_vec().try_into())
            .transpose()
            .map_err(|_| eyre!("Failed to convert ECDSA key"))?
            .ok_or_eyre("No ECDSA key found")?;
        let xt = api::tx().services().register(
            env.blueprint_id,
            services::OperatorPreferences {
                key: ecdsa::Public(ecdsa_key),
                approval: services::ApprovalPrefrence::None,
            },
            Default::default(),
        );
        // send the tx to the tangle and exit.
        tx::tangle::send(&client, &signer, &xt).await?;
    } else {
        // otherwise, we should start the event watcher and run the gadget.
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
    }

    Ok(())
}
