#![allow(unused_results)]

use crate::gossip::{GossipMessage, NetworkService};
use gadget_std::string::ToString;
use gadget_std::sync::atomic::AtomicUsize;
use gadget_std::sync::Arc;
use libp2p::gossipsub::{Event, TopicHash};
use libp2p::{gossipsub, PeerId};

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub(crate) async fn handle_gossip(&mut self, event: gossipsub::Event) {
        let with_connected_peers = |topic: &TopicHash, f: fn(&Arc<AtomicUsize>)| {
            let maybe_mapping = self
                .inbound_mapping
                .iter()
                .find(|r| r.0.to_string() == topic.to_string());
            match maybe_mapping {
                Some((_, _, connected_peers)) => {
                    f(connected_peers);
                    true
                }
                None => false,
            }
        };
        match event {
            Event::Message {
                propagation_source,
                message_id,
                message,
            } => {
                self.handle_gossip_message(propagation_source, message_id, message)
                    .await;
            }
            Event::Subscribed { peer_id, topic } => {
                let added = with_connected_peers(&topic, |_connected_peers| {
                    // Code commented out because each peer needs to do a request-response
                    // direct P2P handshake, which is where the connected_peers counter is
                    // incremented. Adding here will just add twice, which is undesirable.
                    // connected_peers.fetch_add(1, gadget_std::sync::atomic::Ordering::Relaxed);
                });
                if added {
                    gadget_logging::trace!("{peer_id} subscribed to {topic}");
                } else {
                    gadget_logging::error!("{peer_id} subscribed to unknown topic: {topic}");
                }
            }
            Event::Unsubscribed { peer_id, topic } => {
                let removed = with_connected_peers(&topic, |_connected_peers| {
                    // Code commented out because each peer needs to do a request-response
                    // direct P2P handshake, which is where the connected_peers counter is
                    // decremented. Subbing here will just sub twice, which is undesirable.
                    // connected_peers.fetch_sub(1, gadget_std::sync::atomic::Ordering::Relaxed);
                });
                if removed {
                    gadget_logging::trace!("{peer_id} unsubscribed from {topic}");
                } else {
                    gadget_logging::error!("{peer_id} unsubscribed from unknown topic: {topic}");
                }
            }
            Event::GossipsubNotSupported { peer_id } => {
                gadget_logging::trace!("{peer_id} does not support gossipsub!");
            }
            Event::SlowPeer {
                peer_id,
                failed_messages: _,
            } => {
                gadget_logging::error!("{peer_id} wasn't able to download messages in time!");
            }
        }
    }

    #[tracing::instrument(
        skip(self, message),
        fields(
            %_message_id,
            %_propagation_source,
            source = ?message.source
        )
    )]
    async fn handle_gossip_message(
        &mut self,
        _propagation_source: PeerId,
        _message_id: gossipsub::MessageId,
        message: gossipsub::Message,
    ) {
        let Some(origin) = message.source else {
            gadget_logging::error!("Got message from unknown peer");
            return;
        };

        // Reject messages from self
        if origin == self.my_id {
            return;
        }

        gadget_logging::trace!("Got message from peer: {origin}");
        match bincode::deserialize::<GossipMessage>(&message.data) {
            Ok(GossipMessage { topic, raw_payload }) => {
                if let Some((_, tx, _)) = self
                    .inbound_mapping
                    .iter()
                    .find(|r| r.0.to_string() == topic)
                {
                    if let Err(e) = tx.send(raw_payload) {
                        gadget_logging::warn!("Failed to send message to worker: {e}");
                    }
                } else {
                    gadget_logging::error!("No registered worker for topic: {topic}!");
                }
            }
            Err(e) => {
                gadget_logging::error!("Failed to deserialize message (handlers/gossip): {e}");
            }
        }
    }
}
