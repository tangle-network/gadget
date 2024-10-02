//! Runner for Tangle
use parking_lot::RwLock;

use crate::events_watcher::substrate::EventHandler;
use crate::keystore::sp_core_subxt::Pair;
use crate::tangle_subxt::subxt::tx::Signer;
use crate::{
    config::{ContextConfig, GadgetCLICoreSettings, GadgetConfiguration, StdGadgetConfiguration},
    event_listener::{EventListener, IntoTangleEventListener},
    events_watcher::tangle::TangleEventsWatcher,
    info,
    keystore::KeystoreUriSanitizer,
    runner::GadgetRunner,
    tangle_subxt::tangle_testnet_runtime::api::{
        self,
        runtime_types::{
            sp_core::ecdsa,
            tangle_primitives::services::{self, PriceTargets},
        },
    },
    tx,
};

struct TangleGadgetRunner<R>
where
    R: subxt::Config + Send + Sync + 'static,
{
    env: GadgetConfiguration<parking_lot::RawRwLock>,
    price_targets: PriceTargets,
    event_handlers: RwLock<Vec<Box<dyn EventHandler<R>>>>,
}

impl<R> TangleGadgetRunner<R>
where
    R: subxt::Config + Send + Sync + 'static,
{
    pub fn new(
        env: GadgetConfiguration<parking_lot::RawRwLock>,
        price_targets: PriceTargets,
    ) -> Self {
        Self {
            env,
            price_targets,
            event_handlers: RwLock::new(vec![]),
        }
    }

    pub fn set_price_targets(&mut self, price_targets: PriceTargets) {
        self.price_targets = price_targets;
    }
}

#[async_trait::async_trait]
impl<R> GadgetRunner for TangleGadgetRunner<R>
where
    R: subxt::Config + Send + Sync + 'static,
{
    type Error = crate::Error;

    fn config(&self) -> &StdGadgetConfiguration {
        todo!()
    }

    async fn register(&mut self) -> Result<(), Self::Error> {
        // TODO: Use the function in blueprint-test-utils
        if self.env.test_mode {
            info!("Skipping registration in test mode");
            return Ok(());
        }

        let client = self.env.client().await?;
        let ecdsa_pair = self.env.first_ecdsa_signer()?;

        let xt = api::tx().services().register(
            self.env.blueprint_id,
            services::OperatorPreferences {
                key: ecdsa::Public(ecdsa_pair.signer().public().0),
                approval: services::ApprovalPrefrence::None,
                price_targets: self.price_targets.clone(),
            },
            Default::default(),
        );

        // Send the tx to the tangle and exit.
        let signer = self.env.first_sr25519_signer()??;
        let result = tx::tangle::send(&client, &signer, &xt).await?;
        info!("Registered operator with hash: {:?}", result);
        Ok(())
    }

    async fn benchmark(&self) -> std::result::Result<(), Self::Error> {
        todo!()
    }

    async fn run(&self) -> Result<(), Self::Error> {
        let client = self.env.client().await?;
        let signer = self.env.first_sr25519_signer()?;

        info!("Starting the event watcher for {} ...", signer.account_id());

        let event_handlers = self.substrate_event_handlers().read().unwrap().clone();
        let program = TangleEventsWatcher {
            span: self.env.span.clone(),
            client,
            handlers: vec![event_handlers],
        };

        program.into_tangle_event_listener().execute().await;

        Ok(())
    }
}
