//! Tangle Event Watcher Implementation

#![allow(clippy::module_name_repetitions)]
/// An event watcher for the Tangle network.
#[derive(Debug, Default, Clone, Copy)]
pub struct TangleEventsWatcher;

/// A Type alias for the Tangle configuration [`subxt::PolkadotConfig`].
pub type TangleConfig = subxt::PolkadotConfig;

#[async_trait::async_trait]
impl super::substrate::SubstrateEventWatcher<TangleConfig> for TangleEventsWatcher {
    const TAG: &'static str = "tangle";
    const PALLET_NAME: &'static str = "Services";
}
