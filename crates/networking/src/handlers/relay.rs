use crate::network::gossip::NetworkService;
use crate::trace;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub async fn handle_relay_event(&mut self, event: libp2p::relay::Event) {
        trace!("Relay event: {event:?}");
    }

    #[tracing::instrument(skip(self, event))]
    pub async fn handle_relay_client_event(&mut self, event: libp2p::relay::client::Event) {
        trace!("Relay client event: {event:?}");
    }
}
