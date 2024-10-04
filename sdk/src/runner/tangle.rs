//! Runner for Tangle
use parking_lot::RwLock;

use crate::clients::tangle::runtime::TangleConfig;
use crate::events_watcher::substrate;
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

struct TangleGadgetRunner {
    env: GadgetConfiguration<parking_lot::RawRwLock>,
    price_targets: PriceTargets,
    event_handlers: RwLock<Vec<Box<dyn substrate::EventHandler<TangleConfig>>>>,
}

impl TangleGadgetRunner {
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

trait SubstrateGadgetRunner<T>
where
    T: subxt::Config + Send + Sync + 'static,
{
    fn register_event_handler(&self, handler: Box<dyn substrate::EventHandler<T>>);
    fn get_event_handlers(&self) -> Vec<Box<dyn substrate::EventHandler<T>>>;
    fn event_handlers(&self) -> Option<&RwLock<Vec<Box<dyn substrate::EventHandler<T>>>>>;
}

#[async_trait::async_trait]
impl GadgetRunner for TangleGadgetRunner {
    fn config(&self) -> &StdGadgetConfiguration {
        todo!()
    }

    async fn register(&mut self) -> Result<(), crate::Error> {
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
        let signer = self.env.first_sr25519_signer()?;
        let result = tx::tangle::send(&client, &signer, &xt).await?;
        info!("Registered operator with hash: {:?}", result);
        Ok(())
    }

    async fn benchmark(&self) -> std::result::Result<(), crate::Error> {
        todo!()
    }

    async fn run(&self) -> Result<(), crate::Error> {
        let client = self.env.client().await?;
        let signer = self.env.first_sr25519_signer()?;

        info!("Starting the event watcher for {} ...", signer.account_id());

        let event_handlers = self.get_event_handlers();
        let program = TangleEventsWatcher {
            span: self.env.span.clone(),
            client,
            handlers: event_handlers,
        };

        program.into_tangle_event_listener().execute().await;

        Ok(())
    }
}

impl SubstrateGadgetRunner<TangleConfig> for TangleGadgetRunner {
    fn register_event_handler(&self, handler: Box<dyn substrate::EventHandler<TangleConfig>>) {
        self.event_handlers.write().push(handler);
    }

    fn get_event_handlers(&self) -> Vec<Box<dyn substrate::EventHandler<TangleConfig>>> {
        self.event_handlers.read().iter().cloned().collect()
    }

    fn event_handlers(
        &self,
    ) -> Option<&RwLock<Vec<Box<dyn substrate::EventHandler<TangleConfig>>>>> {
        Some(&self.event_handlers)
    }
}
