use crate::gossip::NetworkService;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub(crate) async fn handle_identify_event(&mut self, event: libp2p::identify::Event) {
        use libp2p::identify::Event::{Error, Pushed, Received, Sent};
        match event {
            Received { peer_id, info, .. } => {
                // TODO: Verify the peer info, for example the protocol version, agent version, etc.
                let info_lines = [
                    format!("Protocol Version: {}", info.protocol_version),
                    format!("Agent Version: {}", info.agent_version),
                    format!("Supported Protocols: {:?}", info.protocols),
                ];
                let info_lines = info_lines.join(", ");
                gadget_logging::trace!(
                    "Received identify event from peer: {peer_id} with info: {info_lines}"
                );
                self.swarm.add_external_address(info.observed_addr);
            }
            Sent { peer_id, .. } => {
                gadget_logging::trace!("Sent identify event to peer: {peer_id}");
            }
            Pushed { peer_id, info, .. } => {
                let info_lines = [
                    format!("Protocol Version: {}", info.protocol_version),
                    format!("Agent Version: {}", info.agent_version),
                    format!("Supported Protocols: {:?}", info.protocols),
                ];
                let info_lines = info_lines.join(", ");
                gadget_logging::trace!(
                    "Pushed identify event to peer: {peer_id} with info: {info_lines}"
                );
            }
            Error { peer_id, error, .. } => {
                gadget_logging::error!("Identify error from peer: {peer_id} with error: {error}");
            }
        }
    }
}
