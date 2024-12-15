#![cfg_attr(not(feature = "std"), no_std)]

use async_trait::async_trait;
use auto_impl::auto_impl;
use gadget_std::boxed::Box;

#[cfg(feature = "eigenlayer")]
pub mod eigenlayer;
#[cfg(feature = "evm")]
pub mod evm;
mod network;
#[cfg(feature = "tangle")]
pub mod tangle;

#[async_trait]
#[auto_impl(Arc)]
pub trait Client<Event>: Clone + Send + Sync {
    /// Fetch the next event from the client.
    async fn next_event(&self) -> Option<Event>;
    /// Fetch the latest event from the client.
    ///
    /// If no event has yet been fetched, the client will call [`next_event`](Self::next_event).
    async fn latest_event(&self) -> Option<Event>;
}
