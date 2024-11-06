use crate::network::gossip::NetworkService;
use crate::trace;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub async fn handle_ping_event(&mut self, event: libp2p::ping::Event) {
        trace!("Ping event: {event:?}")
    }
}
