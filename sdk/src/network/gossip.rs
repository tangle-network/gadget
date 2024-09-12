#![allow(
    missing_debug_implementations,
    unused_results,
    clippy::module_name_repetitions,
    clippy::exhaustive_enums
)]
use async_trait::async_trait;
use ecdsa::Public;
use gadget_io::tokio::sync::mpsc::UnboundedSender;
use gadget_io::tokio::sync::{Mutex, RwLock};
use libp2p::gossipsub::IdentTopic;
use libp2p::kad::store::MemoryStore;
use libp2p::{
    gossipsub, mdns, request_response, swarm::NetworkBehaviour, swarm::SwarmEvent, PeerId,
};
use serde::{Deserialize, Serialize};
use sp_core::ecdsa;
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

use crate::error::Error;
use crate::logger::Logger;

use super::{Network, ParticipantInfo, ProtocolMessage};

/// Maximum allowed size for a Signed Message.
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub p2p: request_response::cbor::Behaviour<MyBehaviourRequest, MyBehaviourResponse>,
    pub identify: libp2p::identify::Behaviour,
    pub kadmelia: libp2p::kad::Behaviour<MemoryStore>,
    pub dcutr: libp2p::dcutr::Behaviour,
    pub relay: libp2p::relay::Behaviour,
    pub relay_client: libp2p::relay::client::Behaviour,
    pub ping: libp2p::ping::Behaviour,
}

pub type InboundMapping = (IdentTopic, UnboundedSender<Vec<u8>>, Arc<AtomicU32>);

pub struct NetworkServiceWithoutSwarm<'a> {
    pub logger: &'a Logger,
    pub inbound_mapping: &'a [InboundMapping],
    pub ecdsa_peer_id_to_libp2p_id: Arc<RwLock<HashMap<ecdsa::Public, PeerId>>>,
    pub ecdsa_key: &'a ecdsa::Pair,
    pub span: tracing::Span,
}

impl<'a> NetworkServiceWithoutSwarm<'a> {
    pub(crate) fn with_swarm(
        &'a self,
        swarm: &'a mut libp2p::Swarm<MyBehaviour>,
    ) -> NetworkService<'a> {
        NetworkService {
            swarm,
            logger: self.logger,
            inbound_mapping: self.inbound_mapping,
            ecdsa_peer_id_to_libp2p_id: &self.ecdsa_peer_id_to_libp2p_id,
            ecdsa_key: self.ecdsa_key,
            span: &self.span,
        }
    }
}

pub struct NetworkService<'a> {
    pub swarm: &'a mut libp2p::Swarm<MyBehaviour>,
    pub logger: &'a Logger,
    pub inbound_mapping: &'a [InboundMapping],
    pub ecdsa_peer_id_to_libp2p_id: &'a Arc<RwLock<HashMap<ecdsa::Public, PeerId>>>,
    pub ecdsa_key: &'a ecdsa::Pair,
    pub span: &'a tracing::Span,
}

impl<'a> NetworkService<'a> {
    /// Handle local requests that are meant to be sent to the network.
    pub(crate) fn handle_intra_node_payload(&mut self, msg: IntraNodePayload) {
        let _enter = self.span.enter();
        match (msg.message_type, msg.payload) {
            (MessageType::Broadcast, GossipOrRequestResponse::Gossip(payload)) => {
                let gossip_message = bincode::serialize(&payload).expect("Should serialize");
                if let Err(e) = self
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(msg.topic, gossip_message)
                {
                    self.logger.error(format!("Publish error: {e:?}"));
                }
            }

            (MessageType::P2P(peer_id), GossipOrRequestResponse::Request(req)) => {
                // Send the outer payload in order to attach the topic to it
                // "Requests are sent using Behaviour::send_request and the responses
                // received as Message::Response via Event::Message."
                self.swarm.behaviour_mut().p2p.send_request(&peer_id, req);
            }
            (MessageType::Broadcast, GossipOrRequestResponse::Request(_)) => {
                self.logger.error("Broadcasting a request is not supported");
            }
            (MessageType::Broadcast, GossipOrRequestResponse::Response(_)) => {
                self.logger
                    .error("Broadcasting a response is not supported");
            }
            (MessageType::P2P(_), GossipOrRequestResponse::Gossip(_)) => {
                self.logger
                    .error("P2P message should be a request or response");
            }
            (MessageType::P2P(_), GossipOrRequestResponse::Response(_)) => {
                // TODO: Send the response to the peer.
            }
        }
    }

    /// Handle inbound events from the networking layer
    #[allow(clippy::too_many_lines)]
    pub(crate) async fn handle_swarm_event(&mut self, event: SwarmEvent<MyBehaviourEvent>) {
        use MyBehaviourEvent::{
            Dcutr, Gossipsub, Identify, Kadmelia, Mdns, P2p, Ping, Relay, RelayClient,
        };
        use SwarmEvent::{
            Behaviour, ConnectionClosed, ConnectionEstablished, Dialing, ExpiredListenAddr,
            ExternalAddrConfirmed, ExternalAddrExpired, IncomingConnection,
            IncomingConnectionError, ListenerClosed, ListenerError, NewExternalAddrCandidate,
            NewExternalAddrOfPeer, NewListenAddr, OutgoingConnectionError,
        };
        let _enter = self.span.enter();
        match event {
            Behaviour(P2p(event)) => {
                self.handle_p2p(event).await;
            }
            Behaviour(Gossipsub(event)) => {
                self.handle_gossip(event).await;
            }
            Behaviour(Mdns(event)) => {
                self.handle_mdns_event(event).await;
            }
            Behaviour(Identify(event)) => {
                self.handle_identify_event(event).await;
            }
            Behaviour(Kadmelia(event)) => {
                self.logger.trace(format!("Kadmelia event: {event:?}"));
            }
            Behaviour(Dcutr(event)) => {
                self.handle_dcutr_event(event).await;
            }
            Behaviour(Relay(event)) => {
                self.handle_relay_event(event).await;
            }
            Behaviour(RelayClient(event)) => {
                self.handle_relay_client_event(event).await;
            }
            Behaviour(Ping(event)) => {
                self.handle_ping_event(event).await;
            }

            NewListenAddr {
                address,
                listener_id,
            } => {
                self.logger
                    .debug(format!("{listener_id} has a new address: {address}"));
            }
            ConnectionEstablished {
                peer_id,
                num_established,
                ..
            } => {
                self.handle_connection_established(peer_id, num_established.get())
                    .await;
            }
            ConnectionClosed {
                peer_id,
                num_established,
                cause,
                ..
            } => {
                self.handle_connection_closed(peer_id, num_established, cause)
                    .await;
            }
            IncomingConnection {
                connection_id,
                local_addr,
                send_back_addr,
            } => {
                self.handle_incoming_connection(connection_id, local_addr, send_back_addr)
                    .await;
            }
            IncomingConnectionError {
                connection_id,
                local_addr,
                send_back_addr,
                error,
            } => {
                self.handle_incoming_connection_error(
                    connection_id,
                    local_addr,
                    send_back_addr,
                    error,
                )
                .await;
            }
            OutgoingConnectionError {
                connection_id,
                peer_id,
                error,
            } => {
                self.handle_outgoing_connection_error(connection_id, peer_id, error)
                    .await;
            }
            ExpiredListenAddr {
                listener_id,
                address,
            } => {
                self.logger
                    .trace(format!("{listener_id} has an expired address: {address}"));
            }
            ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => {
                self.logger.trace(format!(
                    "{listener_id} on {addresses:?} has been closed: {reason:?}"
                ));
            }
            ListenerError { listener_id, error } => {
                self.logger
                    .error(format!("{listener_id} has an error: {error}"));
            }
            Dialing {
                peer_id,
                connection_id,
            } => {
                self.logger.debug(format!(
                    "Dialing peer: {peer_id:?} with connection_id: {connection_id}"
                ));
            }
            NewExternalAddrCandidate { address } => {
                self.logger
                    .trace(format!("New external address candidate: {address}"));
            }
            ExternalAddrConfirmed { address } => {
                self.logger
                    .trace(format!("External address confirmed: {address}"));
            }
            ExternalAddrExpired { address } => {
                self.logger
                    .trace(format!("External address expired: {address}"));
            }
            NewExternalAddrOfPeer { peer_id, address } => {
                self.logger.trace(format!(
                    "New external address of peer: {peer_id} with address: {address}"
                ));
            }
            unknown => {
                self.logger
                    .warn(format!("Unknown swarm event: {unknown:?}"));
            }
        }
    }
}

#[derive(Clone)]
pub struct GossipHandle {
    pub topic: IdentTopic,
    pub tx_to_outbound: UnboundedSender<IntraNodePayload>,
    pub rx_from_inbound: Arc<Mutex<gadget_io::tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>>,
    pub logger: Logger,
    pub connected_peers: Arc<AtomicU32>,
    pub ecdsa_peer_id_to_libp2p_id: Arc<RwLock<HashMap<ecdsa::Public, PeerId>>>,
}

impl GossipHandle {
    #[must_use]
    pub fn connected_peers(&self) -> usize {
        self.connected_peers
            .load(std::sync::atomic::Ordering::Relaxed) as usize
    }

    #[must_use]
    pub fn topic(&self) -> IdentTopic {
        self.topic.clone()
    }
}

pub struct IntraNodePayload {
    topic: IdentTopic,
    payload: GossipOrRequestResponse,
    message_type: MessageType,
}

impl std::fmt::Debug for IntraNodePayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IntraNodePayload")
            .field("topic", &self.topic)
            .finish_non_exhaustive()
    }
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug)]
pub enum GossipOrRequestResponse {
    Gossip(GossipMessage),
    Request(MyBehaviourRequest),
    Response(MyBehaviourResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GossipMessage {
    pub topic: String,
    pub raw_payload: Vec<u8>,
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug)]
pub enum MyBehaviourRequest {
    Handshake {
        ecdsa_public_key: ecdsa::Public,
        signature: ecdsa::Signature,
    },
    Message {
        topic: String,
        raw_payload: Vec<u8>,
    },
}

#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug)]
pub enum MyBehaviourResponse {
    Handshaked {
        ecdsa_public_key: ecdsa::Public,
        signature: ecdsa::Signature,
    },
    MessageHandled,
}

enum MessageType {
    Broadcast,
    P2P(PeerId),
}

#[async_trait]
impl Network for GossipHandle {
    async fn next_message(&self) -> Option<ProtocolMessage> {
        let mut lock = self
            .rx_from_inbound
            .try_lock()
            .expect("There should be only a single caller for `next_message`");

        let message = lock.recv().await?;
        match bincode::deserialize(&message) {
            Ok(message) => Some(message),
            Err(e) => {
                self.logger
                    .error(format!("Failed to deserialize message: {e}"));
                drop(lock);
                Network::next_message(self).await
            }
        }
    }

    async fn send_message(&self, message: ProtocolMessage) -> Result<(), Error> {
        let message_type = if let Some(ParticipantInfo {
            ecdsa_key: Some(to),
            ..
        }) = message.recipient
        {
            let libp2p_id = self
                .ecdsa_peer_id_to_libp2p_id
                .read()
                .await
                .get(&Public::from_raw(to.0))
                .copied()
                .ok_or_else(|| Error::Network {
                    reason: format!(
                        "No libp2p ID found for ecdsa public key: {to}. No handshake happened?"
                    ),
                })?;

            MessageType::P2P(libp2p_id)
        } else {
            MessageType::Broadcast
        };

        let payload_inner = match message_type {
            MessageType::Broadcast => GossipOrRequestResponse::Gossip(GossipMessage {
                topic: self.topic.to_string(),
                raw_payload: bincode::serialize(&message).expect("Should serialize"),
            }),
            MessageType::P2P(_) => GossipOrRequestResponse::Request(MyBehaviourRequest::Message {
                topic: self.topic.to_string(),
                raw_payload: bincode::serialize(&message).expect("Should serialize"),
            }),
        };

        let payload = IntraNodePayload {
            topic: self.topic.clone(),
            payload: payload_inner,
            message_type,
        };

        self.tx_to_outbound
            .send(payload)
            .map_err(|e| Error::Network {
                reason: format!("Failed to send intra-node payload: {e}"),
            })
    }
}
