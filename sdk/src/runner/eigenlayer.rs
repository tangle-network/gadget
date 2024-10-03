//! Runner for Eigenlayer

use core::any::Any;

use crate::config::GadgetConfiguration;
use crate::events_watcher::evm::{self, Config};
use crate::info;
use crate::runner::GadgetRunner;
use crate::runner::StdGadgetConfiguration;
use parking_lot::RwLock;

pub trait Contract<T: Config>:
    Deref<Target = alloy_contract::ContractInstance<T::T, T::P, Ethereum>> + Send + Sync + 'static
{
}
pub trait Event: SolEvent + Clone + Send + Sync + 'static {}

// Define a trait for type-erased event handlers
pub trait AnyEventHandler: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn handle_event(&self, event: &dyn Any);
}

// Implement AnyEventHandler for concrete event handlers
impl<F> AnyEventHandler for F
where
    F: Fn(&dyn Any) + Send + Sync + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn handle_event(&self, event: &dyn Any) {
        self(event);
    }
}

trait EvmGadgetRunner<T: Config> {
    fn register_event_handler<C, E>(&self, handler: Box<dyn Fn(&E) + Send + Sync>)
    where
        C: Contract<T>,
        E: Event;

    fn get_event_handlers(&self) -> Vec<Arc<dyn AnyEventHandler>>;
}

pub struct EigenlayerGadgetRunner {
    env: GadgetConfiguration<parking_lot::RawRwLock>,
    event_handlers: RwLock<Vec<Arc<dyn AnyEventHandler>>>,
}

impl<T: Config> EvmGadgetRunner<T> for EigenlayerGadgetRunner {
    fn register_event_handler<C, E>(&self, handler: Box<dyn Fn(&E) + Send + Sync>)
    where
        C: Contract,
        E: Event,
    {
        let any_handler: Arc<dyn AnyEventHandler> = Arc::new(move |event: &dyn Any| {
            if let Some(concrete_event) = event.downcast_ref::<E>() {
                handler(concrete_event);
            }
        });
        self.event_handlers.write().push(any_handler);
    }

    fn get_event_handlers(&self) -> Vec<Arc<dyn AnyEventHandler>> {
        self.event_handlers.read().clone()
    }
}

#[async_trait::async_trait]
impl GadgetRunner for EigenlayerGadgetRunner {
    async fn register(&mut self) -> Result<(), crate::Error> {
        // Implement Eigenlayer-specific registration logic
        todo!()
    }

    async fn run(&self) -> Result<(), crate::Error> {
        let client = self.config().client().await?;

        info!("Starting the event watcher for Eigenlayer...");

        Ok(())
    }

    fn config(&self) -> &StdGadgetConfiguration {
        todo!()
    }

    async fn benchmark(&self) -> std::result::Result<(), crate::Error> {
        todo!()
    }
}
