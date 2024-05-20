use crate::network::gossip::NetworkService;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub async fn handle_dcutr_event(&mut self, event: libp2p::dcutr::Event) {
        self.logger.debug(format!("DCUTR event: {event:?}"));
    }
}
