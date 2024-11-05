#![allow(unused_results)]

use crate::network::gossip::{GossipMessage, NetworkService};

use crate::{error, trace};
use libp2p::gossipsub::TopicHash;
use libp2p::{gossipsub, PeerId};
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

impl NetworkService<'_> {
    #[tracing::instrument(skip(self, event))]
    pub(crate) async fn handle_gossip(&mut self, event: gossipsub::Event) {
        use gossipsub::Event::{GossipsubNotSupported, Message, Subscribed, Unsubscribed};
        let with_connected_peers = |topic: &TopicHash, f: fn(&Arc<AtomicU32>)| {
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
            Message {
                propagation_source,
                message_id,
                message,
            } => {
                self.handle_gossip_message(propagation_source, message_id, message)
                    .await;
            }
            Subscribed { peer_id, topic } => {
                let added = with_connected_peers(&topic, |connected_peers| {
                    connected_peers.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                });
                if added {
                    trace!("{peer_id} subscribed to {topic}");
                } else {
                    error!("{peer_id} subscribed to unknown topic: {topic}");
                }
            }
            Unsubscribed { peer_id, topic } => {
                let removed = with_connected_peers(&topic, |connected_peers| {
                    connected_peers.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                });
                if removed {
                    trace!("{peer_id} unsubscribed from {topic}");
                } else {
                    error!("{peer_id} unsubscribed from unknown topic: {topic}");
                }
            }
            GossipsubNotSupported { peer_id } => {
                trace!("{peer_id} does not support gossipsub!");
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
            error!("Got message from unknown peer");
            return;
        };
        trace!("Got message from peer: {origin}");
        match bincode::deserialize::<GossipMessage>(&message.data) {
            Ok(GossipMessage { topic, raw_payload }) => {
                if let Some((_, tx, _)) = self
                    .inbound_mapping
                    .iter()
                    .find(|r| r.0.to_string() == topic)
                {
                    if let Err(e) = tx.send(raw_payload) {
                        error!("Failed to send message to worker: {e}");
                    }
                } else {
                    error!("No registered worker for topic: {topic}!");
                }
            }
            Err(e) => {
                error!("Failed to deserialize message: {e}");
            }
        }
    }
}
