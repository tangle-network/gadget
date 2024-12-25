#![allow(
    missing_debug_implementations,
    unused_results,
    clippy::module_name_repetitions,
    clippy::exhaustive_enums
)]

use crate::key_types::{KeyPair, PublicKey, Signature};
use crate::Error;
use async_trait::async_trait;
use gadget_crypto::hashing::blake3_256;
use gadget_std::collections::BTreeMap;
use gadget_std::string::ToString;
use gadget_std::sync::atomic::AtomicUsize;
use gadget_std::sync::Arc;
use libp2p::gossipsub::IdentTopic;
use libp2p::kad::store::MemoryStore;
use libp2p::{
    gossipsub, mdns, request_response, swarm::NetworkBehaviour, swarm::SwarmEvent, PeerId,
};
use lru_mem::LruCache;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{Mutex, RwLock};

use crate::networking::{Network, ParticipantInfo, ProtocolMessage};

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

pub type InboundMapping = (IdentTopic, UnboundedSender<Vec<u8>>, Arc<AtomicUsize>);

pub struct NetworkServiceWithoutSwarm<'a> {
    pub inbound_mapping: &'a [InboundMapping],
    pub public_key_to_libp2p_id: Arc<RwLock<BTreeMap<PublicKey, PeerId>>>,
    pub secret_key: &'a KeyPair,
    pub connected_peers: Arc<AtomicUsize>,
    pub span: tracing::Span,
    pub my_id: PeerId,
}

impl<'a> NetworkServiceWithoutSwarm<'a> {
    pub(crate) fn with_swarm(
        &'a self,
        swarm: &'a mut libp2p::Swarm<MyBehaviour>,
    ) -> NetworkService<'a> {
        NetworkService {
            swarm,
            inbound_mapping: self.inbound_mapping,
            public_key_to_libp2p_id: &self.public_key_to_libp2p_id,
            secret_key: self.secret_key,
            connected_peers: self.connected_peers.clone(),
            span: &self.span,
            my_id: self.my_id,
        }
    }
}

pub struct NetworkService<'a> {
    pub swarm: &'a mut libp2p::Swarm<MyBehaviour>,
    pub inbound_mapping: &'a [InboundMapping],
    pub public_key_to_libp2p_id: &'a Arc<RwLock<BTreeMap<PublicKey, PeerId>>>,
    pub connected_peers: Arc<AtomicUsize>,
    pub secret_key: &'a KeyPair,
    pub span: &'a tracing::Span,
    pub my_id: PeerId,
}

impl NetworkService<'_> {
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
                    gadget_logging::error!("Publish error: {e:?}");
                }
            }

            (MessageType::P2P(peer_id), GossipOrRequestResponse::Request(req)) => {
                // Send the outer payload in order to attach the topic to it
                // "Requests are sent using Behaviour::send_request and the responses
                // received as Message::Response via Event::Message."
                self.swarm.behaviour_mut().p2p.send_request(&peer_id, req);
            }
            (MessageType::Broadcast, GossipOrRequestResponse::Request(_)) => {
                gadget_logging::error!("Broadcasting a request is not supported");
            }
            (MessageType::Broadcast, GossipOrRequestResponse::Response(_)) => {
                gadget_logging::error!("Broadcasting a response is not supported");
            }
            (MessageType::P2P(_), GossipOrRequestResponse::Gossip(_)) => {
                gadget_logging::error!("P2P message should be a request or response");
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
                gadget_logging::trace!("Kadmelia event: {event:?}");
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
                gadget_logging::trace!("{listener_id} has a new address: {address}");
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
                gadget_logging::trace!("{listener_id} has an expired address: {address}");
            }
            ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => {
                gadget_logging::trace!(
                    "{listener_id} on {addresses:?} has been closed: {reason:?}"
                );
            }
            ListenerError { listener_id, error } => {
                gadget_logging::error!("{listener_id} has an error: {error}");
            }
            Dialing {
                peer_id,
                connection_id,
            } => {
                gadget_logging::trace!(
                    "Dialing peer: {peer_id:?} with connection_id: {connection_id}"
                );
            }
            NewExternalAddrCandidate { address } => {
                gadget_logging::trace!("New external address candidate: {address}");
            }
            ExternalAddrConfirmed { address } => {
                gadget_logging::trace!("External address confirmed: {address}");
            }
            ExternalAddrExpired { address } => {
                gadget_logging::trace!("External address expired: {address}");
            }
            NewExternalAddrOfPeer { peer_id, address } => {
                gadget_logging::trace!(
                    "New external address of peer: {peer_id} with address: {address}"
                );
            }
            unknown => {
                gadget_logging::warn!("Unknown swarm event: {unknown:?}");
            }
        }
    }
}

pub struct GossipHandle {
    pub topic: IdentTopic,
    pub tx_to_outbound: UnboundedSender<IntraNodePayload>,
    pub rx_from_inbound: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>>,
    pub connected_peers: Arc<AtomicUsize>,
    pub public_key_to_libp2p_id: Arc<RwLock<BTreeMap<PublicKey, PeerId>>>,
    pub recent_messages: parking_lot::Mutex<LruCache<[u8; 32], ()>>,
    pub my_id: PublicKey,
}

impl GossipHandle {
    #[must_use]
    pub fn connected_peers(&self) -> usize {
        self.connected_peers
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    #[must_use]
    pub fn topic(&self) -> IdentTopic {
        self.topic.clone()
    }

    /// Returns an ordered vector of public keys of the peers that are connected to the gossipsub topic.
    pub async fn peers(&self) -> Vec<PublicKey> {
        self.public_key_to_libp2p_id
            .read()
            .await
            .keys()
            .copied()
            .collect()
    }
}

pub struct IntraNodePayload {
    topic: IdentTopic,
    payload: GossipOrRequestResponse,
    message_type: MessageType,
}

impl gadget_std::fmt::Debug for IntraNodePayload {
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
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
        public_key: PublicKey,
        signature: Signature,
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
        public_key: PublicKey,
        signature: Signature,
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
        loop {
            let mut lock = self
                .rx_from_inbound
                .try_lock()
                .expect("There should be only a single caller for `next_message`");

            let message_bytes = lock.recv().await?;
            drop(lock);
            match bincode::deserialize::<ProtocolMessage>(&message_bytes) {
                Ok(message) => {
                    let hash = blake3_256(&message_bytes);
                    let mut map = self.recent_messages.lock();
                    if map
                        .insert(hash, ())
                        .expect("Should not exceed memory limit (rx)")
                        .is_none()
                    {
                        return Some(message);
                    }
                }
                Err(e) => {
                    gadget_logging::error!("Failed to deserialize message (gossip): {e}");
                }
            }
        }
    }

    async fn send_message(&self, mut message: ProtocolMessage) -> Result<(), Error> {
        message.sender.public_key = Some(self.my_id);
        let message_type = if let Some(ParticipantInfo {
            public_key: Some(to),
            ..
        }) = message.recipient
        {
            let pub_key_to_libp2p_id = self.public_key_to_libp2p_id.read().await;
            gadget_logging::trace!("Handshake count: {}", pub_key_to_libp2p_id.len());
            let libp2p_id = pub_key_to_libp2p_id
                .get(&to)
                .copied()
                .ok_or_else(|| {
                    Error::NetworkError(format!(
                        "No libp2p ID found for crypto public key: {:?}. No handshake happened? Total handshakes: {}",
                        to, pub_key_to_libp2p_id.len(),
                    ))
                })?;

            MessageType::P2P(libp2p_id)
        } else {
            MessageType::Broadcast
        };

        let raw_payload =
            bincode::serialize(&message).map_err(|err| Error::MessagingError(err.to_string()))?;
        let payload_inner = match message_type {
            MessageType::Broadcast => GossipOrRequestResponse::Gossip(GossipMessage {
                topic: self.topic.to_string(),
                raw_payload,
            }),
            MessageType::P2P(_) => GossipOrRequestResponse::Request(MyBehaviourRequest::Message {
                topic: self.topic.to_string(),
                raw_payload,
            }),
        };

        let payload = IntraNodePayload {
            topic: self.topic.clone(),
            payload: payload_inner,
            message_type,
        };

        self.tx_to_outbound
            .send(payload)
            .map_err(|e| Error::NetworkError(format!("Failed to send intra-node payload: {e}")))
    }

    fn public_id(&self) -> PublicKey {
        self.my_id
    }
}
