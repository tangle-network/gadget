use crate::events_watcher::evm::{Config as ConfigT, EventWatcher};
use crate::events_watcher::substrate::SubstrateEventWatcher;
use alloy_network::Ethereum;
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

pub struct GenericEventListener<Event: Send + Sync + 'static, Ext: Send + Sync + 'static = ()> {
    listener: Box<dyn EventListener<Event>>,
    _pd: std::marker::PhantomData<Ext>,
}

#[async_trait]
impl<Event: Send + Sync + 'static, Ext: Send + Sync + 'static> EventListener<Event>
    for GenericEventListener<Event, Ext>
{
    async fn next_event(&mut self) -> Option<Event> {
        self.listener.next_event().await
    }

    async fn handle_event(&mut self, event: Event) -> std::io::Result<()> {
        self.listener.handle_event(event).await
    }
}

impl<Event: Send + Sync + 'static, Ext: Send + Sync + 'static> GenericEventListener<Event, Ext> {
    pub fn new<T: EventListener<Event>>(listener: T) -> Self {
        Self {
            listener: Box::new(listener),
            _pd: std::marker::PhantomData,
        }
    }
}

struct EthereumWatcherWrapper<T: EventWatcher<C>, C: ConfigT<N = Ethereum>> {
    listener: T,
    _phantom: std::marker::PhantomData<C>,
}

struct SubstrateWatcherWrapper<
    T: SubstrateEventWatcher<C>,
    C: subxt::Config + Send + Sync + 'static,
> {
    listener: T,
    _phantom: std::marker::PhantomData<C>,
}

impl<Config: ConfigT<N = Ethereum>, Watcher: EventWatcher<Config>> From<Watcher>
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
#[async_trait::async_trait]
impl<Config: ConfigT<N = Ethereum>, Watcher: EventWatcher<Config>> EventListener<Watcher::Event>
    for EthereumWatcherWrapper<Watcher, Config>
{
    async fn next_event(&mut self) -> Option<Watcher::Event> {
        if let Err(err) = self.listener.run().await {
            crate::error!("Error running event watcher: {err}");
        }

        None
    }

    // No-op for now since logic already implemented inside Watcher::run
    async fn handle_event(&mut self, _event: Watcher::Event) -> std::io::Result<()> {
        unreachable!("This function should not be called")
    }
}

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
// TODO: EventListener<Watcher> for SubstrateWatcherWrapper<Watcher, Config> should be:
// TODO: EventListener<Watcher::Event> for SubstrateWatcherWrapper<Watcher::Event, Config>,
// TODO: as well as handle_event and next_event types changed to Watcher::Event
#[async_trait]
impl<Config: subxt::Config + Send + Sync + 'static, Watcher: SubstrateEventWatcher<Config>>
    EventListener<Watcher> for SubstrateWatcherWrapper<Watcher, Config>
{
    async fn next_event(&mut self) -> Option<Watcher> {
        if let Err(err) = self.listener.run().await {
            crate::error!("Error running event watcher: {err}");
        }

        None
    }

    // No-op for now since logic already implemented inside Watcher::run
    async fn handle_event(&mut self, _event: Watcher) -> std::io::Result<()> {
        unreachable!("This function should not be called")
    }
}

pub trait IntoTangleEventListener<Watcher: Send + Sync + 'static, Ext: Send + Sync + 'static> {
    fn into_tangle_event_listener(self) -> GenericEventListener<Watcher, Ext>;
}

impl<Config: subxt::Config + Send + Sync + 'static, Watcher: SubstrateEventWatcher<Config>>
    IntoTangleEventListener<Watcher, Config> for Watcher
{
    fn into_tangle_event_listener(self) -> GenericEventListener<Watcher, Config> {
        let wrapper = SubstrateWatcherWrapper::from(self);
        GenericEventListener::new(wrapper)
    }
}

pub trait IntoEvmEventListener<Event: Send + Sync + 'static, Ext: Send + Sync + 'static = ()> {
    fn into_evm_event_listener(self) -> GenericEventListener<Event, Ext>;
}

impl<Config: ConfigT<N = Ethereum>, Watcher: EventWatcher<Config>>
    IntoEvmEventListener<Watcher::Event, (Config, Watcher)> for Watcher
{
    fn into_evm_event_listener(self) -> GenericEventListener<Watcher::Event, (Config, Watcher)> {
        GenericEventListener::new(EthereumWatcherWrapper::from(self))
    }
}
