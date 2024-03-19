use crate::network::gossip::NetworkService;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub async fn handle_relay_event(&mut self, event: libp2p::relay::Event) {
        self.logger.debug(format!("Relay event: {event:?}"));
    }

    #[tracing::instrument(skip(self, event))]
    pub async fn handle_relay_client_event(&mut self, event: libp2p::relay::client::Event) {
        self.logger.debug(format!("Relay client event: {event:?}"));
    }
}
