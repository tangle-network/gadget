use crate::gossip::NetworkService;
use libp2p::mdns;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub(crate) async fn handle_mdns_event(&mut self, event: mdns::Event) {
        use mdns::Event::{Discovered, Expired};
        match event {
            Discovered(list) => {
                for (peer_id, multiaddr) in list {
                    gadget_logging::trace!("discovered a new peer: {peer_id} on {multiaddr}");
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .add_explicit_peer(&peer_id);
                    if let Err(err) = self.swarm.dial(multiaddr) {
                        gadget_logging::error!("Failed to dial peer: {err}");
                    }
                }
            }
            Expired(list) => {
                for (peer_id, multiaddr) in list {
                    gadget_logging::trace!("discover peer has expired: {peer_id} with {multiaddr}");
                    self.swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                }
            }
        }
    }
}
