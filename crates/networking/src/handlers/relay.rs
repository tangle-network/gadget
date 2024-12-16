use crate::gossip::NetworkService;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub async fn handle_relay_event(&mut self, event: libp2p::relay::Event) {
        gadget_logging::trace!("Relay event: {event:?}");
    }

    #[tracing::instrument(skip(self, event))]
    pub async fn handle_relay_client_event(&mut self, event: libp2p::relay::client::Event) {
        gadget_logging::trace!("Relay client event: {event:?}");
    }
}
