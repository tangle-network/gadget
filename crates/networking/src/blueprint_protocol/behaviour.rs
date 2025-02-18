use crate::{Curve, InstanceMsgKeyPair, InstanceMsgPublicKey, InstanceSignedMsgSignature, KeySignExt};
use dashmap::{DashMap, DashSet};
use gadget_crypto::{hashing::blake3_256, KeyType};
use gadget_logging::{debug, trace, warn};
use libp2p::{
    core::transport::PortUse,
    gossipsub::{self, IdentTopic, MessageAuthenticity, MessageId, Sha256Topic},
    identity::Keypair,
    request_response::{self, OutboundRequestId, ResponseChannel},
    swarm::{
        ConnectionDenied, ConnectionId, FromSwarm, NetworkBehaviour, THandler, THandlerInEvent,
        THandlerOutEvent, ToSwarm,
    },
    Multiaddr, PeerId, StreamProtocol,
};
use std::{
    sync::Arc,
    task::Poll,
    time::{Duration, Instant},
};

use crate::discovery::PeerManager;

use super::{InstanceMessageRequest, InstanceMessageResponse};

#[derive(NetworkBehaviour)]
pub struct DerivedBlueprintProtocolBehaviour {
    /// Request/response protocol for p2p messaging
    request_response:
        request_response::cbor::Behaviour<InstanceMessageRequest, InstanceMessageResponse>,
    /// Gossipsub for broadcast messaging
    gossipsub: gossipsub::Behaviour,
}

/// Events emitted by the BlueprintProtocolBehaviour
#[derive(Debug)]
pub enum BlueprintProtocolEvent {
    /// Request received from a peer
    Request {
        peer: PeerId,
        request: InstanceMessageRequest,
        channel: ResponseChannel<InstanceMessageResponse>,
    },
    /// Response received from a peer
    Response {
        peer: PeerId,
        request_id: OutboundRequestId,
        response: InstanceMessageResponse,
    },
    /// Gossip message received
    GossipMessage {
        source: PeerId,
        message: Vec<u8>,
        topic: IdentTopic,
    },
}

/// Behaviour that handles the blueprint protocol request/response and gossip
pub struct BlueprintProtocolBehaviour {
    /// Request/response protocol for direct messaging
    blueprint_protocol: DerivedBlueprintProtocolBehaviour,
    /// Peer manager for tracking peer states
    pub(crate) peer_manager: Arc<PeerManager>,
    /// Libp2p peer ID
    pub(crate) local_peer_id: PeerId,
    /// Instance public key for handshakes and blueprint_protocol
    pub(crate) instance_public_key: InstanceMsgPublicKey,
    /// Instance secret key for handshakes and blueprint_protocol
    pub(crate) instance_secret_key: InstanceMsgKeyPair,
    /// Peers with pending inbound handshakes
    pub(crate) inbound_handshakes: DashMap<PeerId, Instant>,
    /// Peers with pending outbound handshakes
    pub(crate) outbound_handshakes: DashMap<PeerId, Instant>,
    /// Active response channels
    pub(crate) response_channels:
        DashMap<OutboundRequestId, ResponseChannel<InstanceMessageResponse>>,
}

impl BlueprintProtocolBehaviour {
    /// Create a new blueprint protocol behaviour
    pub fn new(
        local_key: &Keypair,
        instance_secret_key: &InstanceMsgKeyPair,
        instance_public_key: &InstanceMsgPublicKey,
        peer_manager: Arc<PeerManager>,
    ) -> Self {
        let protocols = vec![(
            StreamProtocol::new("/gadget/blueprint_protocol/1.0.0"),
            request_response::ProtocolSupport::Full,
        )];

        // Initialize gossipsub with message signing
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .expect("Valid gossipsub config");

        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .expect("Valid gossipsub behaviour");

        let config = request_response::Config::default()
            .with_request_timeout(Duration::from_secs(30))
            .with_max_concurrent_streams(50);

        let blueprint_protocol = DerivedBlueprintProtocolBehaviour {
            request_response: request_response::cbor::Behaviour::new(protocols, config),
            gossipsub,
        };

        let local_peer_id = local_key.public().to_peer_id();

        Self {
            blueprint_protocol,
            peer_manager,
            local_peer_id,
            instance_public_key: instance_public_key.clone(),
            instance_secret_key: instance_secret_key.clone(),
            inbound_handshakes: DashMap::new(),
            outbound_handshakes: DashMap::new(),
            response_channels: DashMap::new(),
        }
    }

    /// Sign a handshake message for a peer
    pub(crate) fn sign_handshake(&self, peer: &PeerId) -> InstanceSignedMsgSignature {
        let msg = peer.to_bytes();
        let msg_hash = blake3_256(&msg);
        self.instance_secret_key.sign_prehash(&msg_hash)
    }

    /// Send a request to a peer
    pub fn send_request(
        &mut self,
        peer: &PeerId,
        request: InstanceMessageRequest,
    ) -> OutboundRequestId {
        debug!(%peer, ?request, "sending request");
        self.blueprint_protocol
            .request_response
            .send_request(peer, request)
    }

    /// Send a response through a response channel
    pub fn send_response(
        &mut self,
        channel: ResponseChannel<InstanceMessageResponse>,
        response: InstanceMessageResponse,
    ) -> Result<(), InstanceMessageResponse> {
        debug!(?response, "sending response");
        self.blueprint_protocol
            .request_response
            .send_response(channel, response)
    }

    /// Subscribe to a gossip topic
    pub fn subscribe(&mut self, topic: &str) -> Result<bool, gossipsub::SubscriptionError> {
        let topic = Sha256Topic::new(topic);
        self.blueprint_protocol.gossipsub.subscribe(&topic)
    }

    /// Publish a message to a gossip topic
    pub fn publish(
        &mut self,
        topic: &str,
        data: impl Into<Vec<u8>>,
    ) -> Result<MessageId, gossipsub::PublishError> {
        let topic = Sha256Topic::new(topic);
        self.blueprint_protocol.gossipsub.publish(topic, data)
    }

    /// Verify and handle a handshake with a peer
    pub fn verify_handshake(
        &self,
        peer: &PeerId,
        public_key: &InstanceMsgPublicKey,
        signature: &InstanceSignedMsgSignature,
    ) -> Result<(), InstanceMessageResponse> {
        let msg = peer.to_bytes();
        let valid = <Curve as KeyType>::verify(public_key, &msg, signature);
        if !valid {
            warn!("Invalid initial handshake signature from peer: {peer}");
            return Err(InstanceMessageResponse::Error {
                code: 400,
                message: "Invalid handshake signature".to_string(),
            });
        }

        trace!("Received valid handshake from peer: {peer}");

        Ok(())
    }

    pub fn handle_handshake(
        &self,
        peer: &PeerId,
        public_key: &InstanceMsgPublicKey,
        signature: &InstanceSignedMsgSignature,
    ) -> Result<(), InstanceMessageResponse> {
        self.verify_handshake(peer, public_key, signature)?;
        self.peer_manager
            .add_peer_id_to_public_key(peer, public_key);

        Ok(())
    }
    /// Handle a failed handshake with a peer
    pub fn handle_handshake_failure(&self, peer: &PeerId, reason: &str) {
        // Update peer info and potentially ban peer
        if let Some(mut peer_info) = self.peer_manager.get_peer_info(peer) {
            peer_info.failures += 1;
            self.peer_manager.update_peer(*peer, peer_info.clone());

            // Ban peer if too many failures
            if peer_info.failures >= 3 {
                self.peer_manager
                    .ban_peer(*peer, reason, Some(Duration::from_secs(300)));
            }
        }
    }
}

impl NetworkBehaviour for BlueprintProtocolBehaviour {
    type ConnectionHandler =
        <DerivedBlueprintProtocolBehaviour as NetworkBehaviour>::ConnectionHandler;

    type ToSwarm = BlueprintProtocolEvent;

    fn handle_established_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        local_addr: &libp2p::Multiaddr,
        remote_addr: &libp2p::Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.blueprint_protocol
            .handle_established_inbound_connection(connection_id, peer, local_addr, remote_addr)
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        addr: &Multiaddr,
        role_override: libp2p::core::Endpoint,
        port_use: PortUse,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.blueprint_protocol
            .handle_established_outbound_connection(
                connection_id,
                peer,
                addr,
                role_override,
                port_use,
            )
    }

    fn handle_pending_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        local_addr: &libp2p::Multiaddr,
        remote_addr: &libp2p::Multiaddr,
    ) -> Result<(), ConnectionDenied> {
        self.blueprint_protocol.handle_pending_inbound_connection(
            connection_id,
            local_addr,
            remote_addr,
        )
    }

    fn handle_pending_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        maybe_peer: Option<PeerId>,
        addresses: &[libp2p::Multiaddr],
        effective_role: libp2p::core::Endpoint,
    ) -> Result<Vec<libp2p::Multiaddr>, ConnectionDenied> {
        self.blueprint_protocol.handle_pending_outbound_connection(
            connection_id,
            maybe_peer,
            addresses,
            effective_role,
        )
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: THandlerOutEvent<Self>,
    ) {
        self.blueprint_protocol
            .on_connection_handler_event(peer_id, connection_id, event)
    }

    fn on_swarm_event(&mut self, event: FromSwarm<'_>) {
        if let FromSwarm::ConnectionEstablished(e) = &event {
            if e.other_established == 0 {
                self.inbound_handshakes.insert(e.peer_id, Instant::now());
            }
        }

        self.blueprint_protocol.on_swarm_event(event)
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        while let Poll::Ready(ev) = self.blueprint_protocol.poll(cx) {
            match ev {
                ToSwarm::GenerateEvent(ev) => match ev {
                    DerivedBlueprintProtocolBehaviourEvent::RequestResponse(
                        blueprint_protocol_event,
                    ) => self.handle_request_response_event(blueprint_protocol_event),
                    DerivedBlueprintProtocolBehaviourEvent::Gossipsub(gossip_event) => {
                        self.handle_gossipsub_event(gossip_event)
                    }
                },
                ToSwarm::Dial { opts } => {
                    return Poll::Ready(ToSwarm::Dial { opts });
                }
                ToSwarm::NotifyHandler {
                    peer_id,
                    handler,
                    event,
                } => {
                    return Poll::Ready(ToSwarm::NotifyHandler {
                        peer_id,
                        handler,
                        event,
                    })
                }
                ToSwarm::CloseConnection {
                    peer_id,
                    connection,
                } => {
                    return Poll::Ready(ToSwarm::CloseConnection {
                        peer_id,
                        connection,
                    })
                }
                ToSwarm::ListenOn { opts } => return Poll::Ready(ToSwarm::ListenOn { opts }),
                ToSwarm::RemoveListener { id } => {
                    return Poll::Ready(ToSwarm::RemoveListener { id })
                }
                ToSwarm::NewExternalAddrCandidate(addr) => {
                    return Poll::Ready(ToSwarm::NewExternalAddrCandidate(addr))
                }
                ToSwarm::ExternalAddrConfirmed(addr) => {
                    return Poll::Ready(ToSwarm::ExternalAddrConfirmed(addr))
                }
                ToSwarm::ExternalAddrExpired(addr) => {
                    return Poll::Ready(ToSwarm::ExternalAddrExpired(addr))
                }
                _ => {}
            }
        }

        Poll::Pending
    }
}
