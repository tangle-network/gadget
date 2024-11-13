use crate::Error;
use async_trait::async_trait;
use std::iter::Take;
use tokio_retry::strategy::ExponentialBackoff;

pub mod evm;
pub mod executor;
pub mod markers;
pub mod periodic;
pub mod tangle;
#[cfg(feature = "testing")]
pub mod testing;

/// The [`EventListener`] trait defines the interface for event listeners.
#[async_trait]
pub trait EventListener<T: Send + 'static, Ctx: Send + 'static>: Send + 'static {
    async fn new(context: &Ctx) -> Result<Self, Error>
    where
        Self: Sized;

    /// Obtains the next event to be processed by the event listener.
    async fn next_event(&mut self) -> Option<T>;
}

pub fn get_exponential_backoff<const N: usize>() -> Take<ExponentialBackoff> {
    ExponentialBackoff::from_millis(2).factor(1000).take(N)
}
