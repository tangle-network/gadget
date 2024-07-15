use std::str::FromStr;

use alloy_primitives::U256;
use color_eyre::{eyre::OptionExt, Result};
use gadget_sdk::{
    events_watcher::{self, tangle::*, EventHandler, SubstrateEventWatcher},
    keystore::Backend,
};
use subxt_signer::ExposeSecret;
use tangle_subxt_v2::{
    subxt,
    tangle_testnet_runtime::api::{
        self as TangleApi,
        runtime_types::{
            bounded_collections::bounded_vec::BoundedVec, tangle_primitives::services::field::Field,
        },
        services::events::{JobCalled, JobResultSubmitted},
    },
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
    let signer = subxt_signer::sr25519::Keypair::from_seed(seed)?;

    let x_square = IncredibleSquaringEventHandler {
        service_id: env.service_id,
        signer,
    };

    SubstrateEventWatcher::run(
        &TangleEventsWatcher,
        client,
        // Add more handler here if we have more functions.
        vec![Box::new(x_square)],
    )
    .await?;

    Ok(())
}

struct IncredibleSquaringEventHandler {
    service_id: u64,
    signer: subxt_signer::sr25519::Keypair,
}

#[async_trait::async_trait]
impl EventHandler<TangleConfig> for IncredibleSquaringEventHandler {
    async fn can_handle_events(
        &self,
        events: subxt::events::Events<TangleConfig>,
    ) -> Result<bool, events_watcher::Error> {
        // filter only the events for our service id and for x square job.
        let has_event = events.find::<JobCalled>().flatten().any(|event| {
            event.service_id == self.service_id && event.job == blueprint::X_SQUARE_JOB_ID
        });

        Ok(has_event)
    }
    async fn handle_events(
        &self,
        client: subxt::OnlineClient<TangleConfig>,
        (events, _block_number): (subxt::events::Events<TangleConfig>, u64),
    ) -> Result<(), events_watcher::Error> {
        let x_square_job_events: Vec<_> = events
            .find::<JobCalled>()
            .flatten()
            .filter(|event| {
                event.service_id == self.service_id && event.job == blueprint::X_SQUARE_JOB_ID
            })
            .collect();
        for call in x_square_job_events {
            let Some(Field::Bytes(x)) = call.args.first() else {
                tracing::warn!("No argument provided for x square job");
                continue;
            };
            let x = U256::from_be_slice(&x.0);
            // do the squaring
            let xsquare = blueprint::xsquare(x);
            let xsquare_bytes = xsquare.to_be_bytes_vec();
            // craft the response.
            let result = vec![Field::Bytes(BoundedVec(xsquare_bytes))];
            let response =
                TangleApi::tx()
                    .services()
                    .submit_result(self.service_id, call.call_id, result);
            let progress = client
                .tx()
                .sign_and_submit_then_watch_default(&response, &self.signer)
                .await?;
            let events = progress.wait_for_finalized_success().await?;
            // find our event.
            let maybe_event = events.find_first::<JobResultSubmitted>()?;
            tracing::debug!("JobResultSubmitted event: {:?}", maybe_event);
        }
        Ok(())
    }
}
