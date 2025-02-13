use crate::gossip::NetworkService;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, _event))]
    pub async fn handle_ping_event(&mut self, _event: libp2p::ping::Event) {
        //gadget_logging::trace!("Ping event: {event:?}");
    }
}
