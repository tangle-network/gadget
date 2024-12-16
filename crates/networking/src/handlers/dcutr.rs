use crate::gossip::NetworkService;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub async fn handle_dcutr_event(&mut self, event: libp2p::dcutr::Event) {
        gadget_logging::trace!("DCUTR event: {event:?}");
    }
}
