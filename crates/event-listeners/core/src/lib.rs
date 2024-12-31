pub mod exponential_backoff;
pub mod marker;

pub mod error;
pub use error::Error;
pub mod executor;
#[cfg(feature = "testing")]
pub mod testing;

use async_trait::async_trait;
use exponential_backoff::ExponentialBackoff;
use gadget_std::iter::Take;

/// The [`EventListener`] trait defines the interface for event listeners.
#[async_trait]
pub trait EventListener<T: Send + 'static, Ctx: Send + 'static>: Send + 'static {
    type ProcessorError: core::error::Error + Send + Sync + 'static;

    async fn new(context: &Ctx) -> Result<Self, Error<Self::ProcessorError>>
    where
        Self: Sized;

    /// Obtains the next event to be processed by the event listener.
    async fn next_event(&mut self) -> Option<T>;
}

pub fn get_exponential_backoff<const N: usize>() -> Take<ExponentialBackoff> {
    ExponentialBackoff::from_millis(2).factor(1000).take(N)
}

#[async_trait]
pub trait InitializableEventHandler {
    async fn init_event_handler(
        &self,
    ) -> Option<tokio::sync::oneshot::Receiver<Result<(), Error<error::Unit>>>>;
}
