#![cfg_attr(not(feature = "std"), no_std)]
pub mod client;
pub mod error;
pub mod services;

#[cfg(not(any(feature = "std", feature = "web")))]
compile_error!("`std` or `web` feature required");

use auto_impl::auto_impl;

#[auto_impl(Arc)]
pub trait EventsClient<Event>: Clone + Send + Sync {
    /// Fetch the next event from the client.
    async fn next_event(&self) -> Option<Event>;
    /// Fetch the latest event from the client.
    ///
    /// If no event has yet been fetched, the client will call [`next_event`](Self::next_event).
    async fn latest_event(&self) -> Option<Event>;
}

// NOTE: Actual client tests are in gadget-tangle-testing-utils
#[test]
fn it_works() {
    assert_eq!(2 + 2, 4);
}
