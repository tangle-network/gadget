//! Tangle Event Watcher Implementation

#![allow(clippy::module_name_repetitions)]

use crate::clients::tangle::runtime::TangleConfig;
use crate::events_watcher::substrate::LoggerEnv;
use crate::logger::Logger;

/// An event watcher for the Tangle network.
#[derive(Debug, Clone)]
pub struct TangleEventsWatcher {
    pub logger: Logger,
}

#[async_trait::async_trait]
impl super::substrate::SubstrateEventWatcher<TangleConfig> for TangleEventsWatcher {
    const TAG: &'static str = "tangle";
    const PALLET_NAME: &'static str = "Services";

    fn logger(&self) -> &Logger {
        &self.logger
    }
}

impl LoggerEnv for TangleEventsWatcher {
    fn logger(&self) -> &Logger {
        &self.logger
    }
}
