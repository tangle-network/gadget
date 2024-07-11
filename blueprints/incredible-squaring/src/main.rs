use std::path::PathBuf;

use alloy_primitives::U256;
use color_eyre::{
    eyre::{bail, Context, OptionExt},
    Result,
};
use gadget_sdk::keystore::{
    backend::{fs::FilesystemKeystore, mem::InMemoryKeystore, GenericKeyStore},
    Backend,
};
use tangle_subxt_v2::{
    subxt::{self, SubstrateConfig},
    tangle_testnet_runtime::api::{
        self as TangleApi,
        runtime_types::{
            bounded_collections::bounded_vec::BoundedVec, tangle_primitives::services::field::Field,
        },
        services::{calls::types, events::JobCalled},
    },
};

use incredible_squaring_blueprint as blueprint;

// TODO: move this to the SDK.
#[derive(Debug, Clone)]
struct GadgetEnvironment {
    tangle_rpc_endpoint: String,
    keystore_uri: String,
    data_dir: PathBuf,
    blueprint_id: u64,
    service_id: u64,
}

impl GadgetEnvironment {
    /// Create a new Operator from the environment.
    fn from_env() -> Result<Self> {
        Ok(Self {
            tangle_rpc_endpoint: std::env::var("RPC_URL").context("loading RPC_URL from env")?,
            keystore_uri: std::env::var("KEYSTORE_URI").context("loading KEYSTORE_URI from env")?,
            data_dir: std::env::var("DATA_DIR")
                .context("loading DATA_DIR from env")?
                .into(),
            blueprint_id: std::env::var("BLUEPRINT_ID")
                .context("loading BLUEPRINT_ID from env")?
                .parse()
                .context("parsing BLUEPRINT_ID not a u64")?,
            service_id: std::env::var("SERVICE_ID")
                .context("loading SERVICE_ID from env")?
                .parse()
                .context("parsing SERVICE_ID not a u64")?,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let env = GadgetEnvironment::from_env()?;

    // TODO: move this part to the SDK.
    let keystore = match env.keystore_uri {
        uri if uri == "file::memory:" || uri == ":memory:" => {
            GenericKeyStore::Mem(InMemoryKeystore::new())
        }
        uri if uri.starts_with("file:") || uri.starts_with("file://") => {
            let path = uri
                .trim_start_matches("file:")
                .trim_start_matches("file://");
            GenericKeyStore::Fs(FilesystemKeystore::open(path)?)
        }
        otherwise => {
            bail!("Unsupported keystore URI: {otherwise}")
        }
    };

    let client =
        subxt::OnlineClient::<subxt::SubstrateConfig>::from_url(&env.tangle_rpc_endpoint).await?;

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
        blueprint_id: env.blueprint_id,
        signer,
    };

    gadget_sdk::events_watcher::SubstrateEventWatcher::run(
        &TangleEventsWatcher,
        client,
        // Add more handler here if we have more functions.
        vec![Box::new(x_square)],
    )
    .await?;

    Ok(())
}

/// An event watcher for the Tangle network.
struct TangleEventsWatcher;

#[async_trait::async_trait]
impl gadget_sdk::events_watcher::SubstrateEventWatcher<SubstrateConfig> for TangleEventsWatcher {
    const TAG: &'static str = "tangle";
    const PALLET_NAME: &'static str = "Services";
}

struct IncredibleSquaringEventHandler {
    service_id: u64,
    blueprint_id: u64,
    signer: subxt_signer::sr25519::Keypair,
}

#[async_trait::async_trait]
impl gadget_sdk::events_watcher::EventHandler<SubstrateConfig> for IncredibleSquaringEventHandler {
    async fn can_handle_events(
        &self,
        events: subxt::events::Events<SubstrateConfig>,
    ) -> Result<bool, gadget_sdk::events_watcher::Error> {
        // filter only the events for our service id and for x square job.
        let has_event = events.find::<JobCalled>().flatten().any(|event| {
            event.service_id == self.service_id && event.job == blueprint::X_SQUARE_JOB_ID
        });

        Ok(has_event)
    }
    async fn handle_events(
        &self,
        client: subxt::OnlineClient<SubstrateConfig>,
        (events, block_number): (subxt::events::Events<SubstrateConfig>, u64),
    ) -> Result<(), gadget_sdk::events_watcher::Error> {
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
        }
        Ok(())
    }
}
