use crate::events_watcher::evm::{Config as ConfigT, EventWatcher};
use crate::events_watcher::substrate::SubstrateEventWatcher;
use crate::Error;
use async_trait::async_trait;

pub mod periodic;

/// The [`EventListener`] trait defines the interface for event listeners.
#[async_trait]
pub trait EventListener<T: Send + Sync + 'static, Ctx: Send + Sync + 'static>:
    Send + Sync + 'static
{
    async fn new(context: &Ctx) -> Result<Self, Error>
    where
        Self: Sized;
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

pub struct GenericEventListener<
    Event: Send + Sync + 'static,
    Ctx: Send + Sync + 'static,
    Ext: Send + Sync + 'static = (),
> {
    listener: Box<dyn EventListener<Event, Ctx>>,
    _pd: std::marker::PhantomData<Ext>,
}

#[async_trait]
impl<Event: Send + Sync + 'static, Ctx: Send + Sync + 'static, Ext: Send + Sync + 'static>
    EventListener<Event, Ctx> for GenericEventListener<Event, Ctx, Ext>
{
    async fn new(_ctx: &Ctx) -> Result<Self, Error>
    where
        Self: Sized,
    {
        unreachable!(
            "This function should not be called directly. This is for internal dev use only"
        )
    }
    async fn next_event(&mut self) -> Option<Event> {
        self.listener.next_event().await
    }

    async fn handle_event(&mut self, event: Event) -> std::io::Result<()> {
        self.listener.handle_event(event).await
    }
}

impl<Event: Send + Sync + 'static, Ctx: Send + Sync + 'static, Ext: Send + Sync + 'static>
    GenericEventListener<Event, Ctx, Ext>
{
    pub fn new<T: EventListener<Event, Ctx>>(listener: T) -> Self {
        Self {
            listener: Box::new(listener),
            _pd: std::marker::PhantomData,
        }
    }
}

struct EthereumWatcherWrapper<T: EventWatcher<C>, Ctx: Send + Sync + 'static, C: ConfigT> {
    listener: T,
    _phantom: std::marker::PhantomData<(Ctx, C)>,
}

struct SubstrateWatcherWrapper<
    T: SubstrateEventWatcher<C>,
    Ctx: Send + Sync + 'static,
    C: subxt::Config + Send + Sync + 'static,
> {
    listener: T,
    _phantom: std::marker::PhantomData<(C, Ctx)>,
}

impl<Config: ConfigT, Ctx: Send + Sync + 'static, Watcher: EventWatcher<Config>> From<Watcher>
    for EthereumWatcherWrapper<Watcher, Ctx, Config>
{
    fn from(value: Watcher) -> Self {
        Self {
            listener: value,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<Config: ConfigT, Ctx: Send + Sync + 'static, Watcher: EventWatcher<Config>>
    EventListener<Watcher::Event, Ctx> for EthereumWatcherWrapper<Watcher, Ctx, Config>
{
    async fn new(_context: &Ctx) -> Result<Self, Error>
    where
        Self: Sized,
    {
        unreachable!("For internal dev use only")
    }

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

impl<
        Config: subxt::Config + Send + Sync + 'static,
        Ctx: Send + Sync + 'static,
        Watcher: SubstrateEventWatcher<Config>,
    > From<Watcher> for SubstrateWatcherWrapper<Watcher, Ctx, Config>
{
    fn from(value: Watcher) -> Self {
        Self {
            listener: value,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<
        Config: subxt::Config + Send + Sync + 'static,
        Ctx: Send + Sync + 'static,
        Watcher: SubstrateEventWatcher<Config>,
    > EventListener<Watcher, Ctx> for SubstrateWatcherWrapper<Watcher, Ctx, Config>
{
    async fn new(_ctx: &Ctx) -> Result<Self, Error>
    where
        Self: Sized,
    {
        unreachable!("For internal use only")
    }

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

pub trait IntoTangleEventListener<
    Watcher: Send + Sync + 'static,
    Ctx: Send + Sync + 'static,
    Ext: Send + Sync + 'static,
>
{
    fn into_tangle_event_listener(self) -> GenericEventListener<Watcher, Ctx, Ext>;
}

impl<Config: subxt::Config + Send + Sync + 'static, Watcher: SubstrateEventWatcher<Config>>
    IntoTangleEventListener<Watcher, (), Config> for Watcher
{
    fn into_tangle_event_listener(self) -> GenericEventListener<Watcher, (), Config> {
        let wrapper = SubstrateWatcherWrapper::<Watcher, (), Config>::from(self);
        GenericEventListener::new(wrapper)
    }
}

pub trait IntoEvmEventListener<
    Event: Send + Sync + 'static,
    Ctx: Send + Sync + 'static,
    Ext: Send + Sync + 'static = (),
>
{
    fn into_evm_event_listener(self) -> GenericEventListener<Event, Ctx, Ext>;
}

impl<Config: ConfigT, Ctx: Send + Sync + 'static, Watcher: EventWatcher<Config>>
    IntoEvmEventListener<Watcher::Event, Ctx, (Config, Watcher)> for Watcher
{
    fn into_evm_event_listener(
        self,
    ) -> GenericEventListener<Watcher::Event, Ctx, (Config, Watcher)> {
        GenericEventListener::new(EthereumWatcherWrapper::from(self))
    }
}
