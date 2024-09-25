use crate::debug;
use crate::network::gossip::NetworkService;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub async fn handle_ping_event(&mut self, event: libp2p::ping::Event) {
        debug!("Ping event: {event:?}")
    }
}
