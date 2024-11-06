use crate::network::gossip::NetworkService;
use crate::trace;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub async fn handle_dcutr_event(&mut self, event: libp2p::dcutr::Event) {
        trace!("DCUTR event: {event:?}");
    }
}
