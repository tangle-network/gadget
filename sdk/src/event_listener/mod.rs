use crate::Error;
use async_trait::async_trait;
use std::iter::Take;
use tokio_retry::strategy::ExponentialBackoff;

pub mod evm_contracts;
pub mod periodic;
pub mod tangle_events;

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
    async fn handle_event(&mut self, event: T) -> Result<(), Error>;

    /// The event loop for the event listener
    async fn execute(&mut self) -> Result<(), Error> {
        while let Some(event) = self.next_event().await {
            if let Err(err) = self.handle_event(event).await {
                crate::error!("Error handling event: {err}");
            }
        }

        crate::warn!("Event listener has stopped");
        Err(Error::Other("Event listener has stopped".to_string()))
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

    async fn handle_event(&mut self, event: Event) -> Result<(), Error> {
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

pub fn get_exponential_backoff<const N: usize>() -> Take<ExponentialBackoff> {
    ExponentialBackoff::from_millis(2).factor(1000).take(N)
}
