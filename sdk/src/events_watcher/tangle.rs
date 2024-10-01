//! Tangle Event Watcher Implementation

#![allow(clippy::module_name_repetitions)]

use crate::clients::tangle::runtime::{TangleClient, TangleConfig};
use crate::events_watcher::substrate::{EventHandler, EventHandlerFor};
use subxt::OnlineClient;

/// An event watcher for the Tangle network.
pub struct TangleEventsWatcher {
    pub span: tracing::Span,
    pub client: TangleClient,
    pub handlers: Vec<Box<dyn EventHandler<TangleConfig>>>,
}

#[async_trait::async_trait]
impl super::substrate::SubstrateEventWatcher<TangleConfig> for TangleEventsWatcher {
    const TAG: &'static str = "tangle";
    const PALLET_NAME: &'static str = "Services";

    fn client(&self) -> &OnlineClient<TangleConfig> {
        &self.client
    }

    fn handlers(&self) -> &Vec<EventHandlerFor<TangleConfig>> {
        &self.handlers
    }
}
