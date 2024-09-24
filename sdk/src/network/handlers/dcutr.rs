use crate::debug;
use crate::network::gossip::NetworkService;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub async fn handle_dcutr_event(&mut self, event: libp2p::dcutr::Event) {
        debug!("DCUTR event: {event:?}");
    }
}
