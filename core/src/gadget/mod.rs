use alloc::boxed::Box;
use async_trait::async_trait;
use core::error::Error;

pub mod error;
pub mod general;
#[cfg(feature = "std")]
pub mod manager;
#[cfg(feature = "substrate")]
pub mod substrate;

#[async_trait]
pub trait AbstractGadget: Send + Sync + 'static {
    type Event: Send + Sync + 'static;
    type ProtocolMessage: Send + Sync + 'static;
    type Error: Error + Send + Sync + 'static;

    async fn next_event(&self) -> Option<Self::Event>;
    async fn get_next_protocol_message(&self) -> Option<Self::ProtocolMessage>;

    async fn on_event_received(&self, notification: Self::Event) -> Result<(), Self::Error>;
    async fn process_protocol_message(
        &self,
        message: Self::ProtocolMessage,
    ) -> Result<(), Self::Error>;

    async fn process_error(&self, error: Self::Error);
}
