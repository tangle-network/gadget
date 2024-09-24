//! Tangle Event Watcher Implementation

#![allow(clippy::module_name_repetitions)]

use crate::clients::tangle::runtime::TangleConfig;

/// An event watcher for the Tangle network.
#[derive(Debug, Clone)]
pub struct TangleEventsWatcher {
    pub span: tracing::Span,
}

#[async_trait::async_trait]
impl super::substrate::SubstrateEventWatcher<TangleConfig> for TangleEventsWatcher {
    const TAG: &'static str = "tangle";
    const PALLET_NAME: &'static str = "Services";
}
