use crate::network::gossip::NetworkService;
use crate::trace;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    async fn handle_kadmelia_event(&mut self, event: libp2p::kad::Event) {
        // TODO: Handle kadmelia events
        trace!("Kadmelia event: {event:?}");
    }
}
