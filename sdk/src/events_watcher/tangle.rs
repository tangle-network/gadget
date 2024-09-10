#![allow(clippy::module_name_repetitions)]

use gadget_common::prelude::DebugLogger;

/// An event watcher for the Tangle network.
#[derive(Debug, Clone)]
pub struct TangleEventsWatcher {
    pub logger: DebugLogger,
}

/// A Type alias for the Tangle configuration [`subxt::PolkadotConfig`].
pub type TangleConfig = subxt::PolkadotConfig;

#[async_trait::async_trait]
impl super::substrate::SubstrateEventWatcher<TangleConfig> for TangleEventsWatcher {
    const TAG: &'static str = "tangle";
    const PALLET_NAME: &'static str = "Services";
    fn logger(&self) -> &DebugLogger {
        &self.logger
    }
}
