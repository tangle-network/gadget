use crate::events_watcher::evm::{Config as ConfigT, EventWatcher};
use crate::events_watcher::substrate::SubstrateEventWatcher;
use async_trait::async_trait;

pub mod periodic;

/// The [`EventListener`] trait defines the interface for event listeners.
#[async_trait]
pub trait EventListener<T: Send + Sync + 'static>: Send + Sync + 'static {
    /// Obtains the next event to be processed by the event listener.
    async fn next_event(&mut self) -> Option<T>;

    /// Logic for handling received events
    async fn handle_event(&mut self, event: T) -> std::io::Result<()>;

    /// The event loop for the event listener
    async fn execute(&mut self) {
        while let Some(event) = self.next_event().await {
            if let Err(err) = self.handle_event(event).await {
                crate::error!("Error handling event: {err}");
            }
        }

        crate::warn!("Event listener has stopped")
    }
}

// TODO: Don't allow dead code once implemented
#[allow(dead_code)]
pub struct EthereumWatcherWrapper<T: EventWatcher<C>, C: ConfigT> {
    listener: T,
    _phantom: std::marker::PhantomData<C>,
}

pub struct SubstrateWatcherWrapper<
    T: SubstrateEventWatcher<C>,
    C: subxt::Config + Send + Sync + 'static,
> {
    listener: T,
    _phantom: std::marker::PhantomData<C>,
}

impl<Config: ConfigT, Watcher: EventWatcher<Config>> From<Watcher>
    for EthereumWatcherWrapper<Watcher, Config>
{
    fn from(value: Watcher) -> Self {
        Self {
            listener: value,
            _phantom: std::marker::PhantomData,
        }
    }
}

// TODO: Refactor up a level of abstraction after tests pass to truly use the EventListener interface
// TODO: Uncomment once generalized
// #[async_trait::async_trait]
// impl<Config: ConfigT, Watcher: EventWatcher<Config>> EventListener<Watcher::Event>
//     for EthereumWatcherWrapper<Watcher, Config>
// {
//     async fn next_event(&mut self) -> Option<Watcher::Event> {
//         if let Err(err) = self.listener.run().await {
//             crate::error!("Error running event watcher: {err}");
//         }
//
//         None
//     }
//
//     // No-op for now since logic already implemented inside Watcher::run
//     async fn handle_event(&mut self, _event: Watcher::Event) -> std::io::Result<()> {
//         unreachable!("This function should not be called")
//     }
// }

impl<Config: subxt::Config + Send + Sync + 'static, Watcher: SubstrateEventWatcher<Config>>
    From<Watcher> for SubstrateWatcherWrapper<Watcher, Config>
{
    fn from(value: Watcher) -> Self {
        Self {
            listener: value,
            _phantom: std::marker::PhantomData,
        }
    }
}

// TODO: Refactor up a level of abstraction after tests pass to truly use the EventListener interface
#[async_trait::async_trait]
impl<Config: subxt::Config + Send + Sync + 'static, Watcher: SubstrateEventWatcher<Config>>
    EventListener<()> for SubstrateWatcherWrapper<Watcher, Config>
{
    async fn next_event(&mut self) -> Option<()> {
        if let Err(err) = self.listener.run().await {
            crate::error!("Error running event watcher: {err}");
        }

        None
    }

    // No-op for now since logic already implemented inside Watcher::run
    async fn handle_event(&mut self, _event: ()) -> std::io::Result<()> {
        unreachable!("This function should not be called")
    }
}
