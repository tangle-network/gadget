use crate::gossip::NetworkService;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    async fn handle_kadmelia_event(&mut self, event: libp2p::kad::Event) {
        // TODO: Handle kadmelia events
        gadget_logging::trace!("Kadmelia event: {event:?}");
    }
}
