use crate::{Curve, InstanceMsgPublicKey, InstanceSignedMsgSignature};
use dashmap::DashMap;
use gadget_crypto::KeyType;
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

/// Events emitted by the BlueprintProtocolBehaviour
#[derive(Debug)]
pub enum BlueprintProtocolEvent {
    /// Response received from a peer
    Response {
        peer: PeerId,
        request_id: OutboundRequestId,
        response: InstanceMessageResponse,
    },
    /// Request received from a peer
    Request {
        peer: PeerId,
        request: InstanceMessageRequest,
        channel: ResponseChannel<InstanceMessageResponse>,
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
    request_response:
        request_response::cbor::Behaviour<InstanceMessageRequest, InstanceMessageResponse>,
    /// Gossipsub for broadcast messaging
    gossipsub: gossipsub::Behaviour,
    /// Peer manager for tracking peer states
    peer_manager: Arc<PeerManager>,
    /// Peers with outstanding handshake requests
    pending_handshake_peers: DashMap<PeerId, Instant>,
    /// Active response channels
    response_channels: DashMap<OutboundRequestId, ResponseChannel<InstanceMessageResponse>>,
}

impl BlueprintProtocolBehaviour {
    /// Create a new blueprint protocol behaviour
    pub fn new(local_key: &Keypair, peer_manager: Arc<PeerManager>) -> Self {
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

        let config = libp2p::request_response::Config::default()
            .with_request_timeout(Duration::from_secs(30))
            .with_max_concurrent_streams(50);

        Self {
            request_response: request_response::cbor::Behaviour::new(protocols, config),
            gossipsub,
            peer_manager,
            pending_handshake_peers: DashMap::new(),
            response_channels: DashMap::new(),
        }
    }

    /// Send a request to a peer
    pub fn send_request(
        &mut self,
        peer: &PeerId,
        request: InstanceMessageRequest,
    ) -> OutboundRequestId {
        debug!(%peer, ?request, "sending request");
        self.request_response.send_request(peer, request)
    }

    /// Send a response through a response channel
    pub fn send_response(
        &mut self,
        channel: ResponseChannel<InstanceMessageResponse>,
        response: InstanceMessageResponse,
    ) -> Result<(), InstanceMessageResponse> {
        debug!(?response, "sending response");
        self.request_response.send_response(channel, response)
    }

    /// Subscribe to a gossip topic
    pub fn subscribe(&mut self, topic: &str) -> Result<bool, gossipsub::SubscriptionError> {
        let topic = Sha256Topic::new(topic);
        self.gossipsub.subscribe(&topic)
    }

    /// Publish a message to a gossip topic
    pub fn publish(
        &mut self,
        topic: &str,
        data: impl Into<Vec<u8>>,
    ) -> Result<MessageId, gossipsub::PublishError> {
        let topic = Sha256Topic::new(topic);
        self.gossipsub.publish(topic, data)
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
    type ConnectionHandler = <request_response::cbor::Behaviour<
        InstanceMessageRequest,
        InstanceMessageResponse,
    > as NetworkBehaviour>::ConnectionHandler;

    type ToSwarm = <request_response::cbor::Behaviour<
        InstanceMessageRequest,
        InstanceMessageResponse,
    > as NetworkBehaviour>::ToSwarm;

    fn handle_established_inbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        local_addr: &libp2p::Multiaddr,
        remote_addr: &libp2p::Multiaddr,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.request_response.handle_established_inbound_connection(
            connection_id,
            peer,
            local_addr,
            remote_addr,
        )
    }

    fn handle_established_outbound_connection(
        &mut self,
        connection_id: ConnectionId,
        peer: PeerId,
        addr: &Multiaddr,
        role_override: libp2p::core::Endpoint,
        port_use: PortUse,
    ) -> Result<THandler<Self>, ConnectionDenied> {
        self.request_response
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
        self.request_response.handle_pending_inbound_connection(
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
        self.request_response.handle_pending_outbound_connection(
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
        self.request_response
            .on_connection_handler_event(peer_id, connection_id, event)
    }

    fn on_swarm_event(&mut self, event: FromSwarm<'_>) {
        if let FromSwarm::ConnectionEstablished(e) = &event {
            if e.other_established == 0 {
                self.pending_handshake_peers
                    .insert(e.peer_id, Instant::now());
            }
        }

        self.request_response.on_swarm_event(event)
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<ToSwarm<Self::ToSwarm, THandlerInEvent<Self>>> {
        if let Poll::Ready(ev) = self.request_response.poll(cx) {
            // Remove a peer from `pending_handshake_peers` when its handshake request is received.
            if let ToSwarm::GenerateEvent(request_response::Event::Message {
                peer,
                message:
                    request_response::Message::Request {
                        request:
                            InstanceMessageRequest::Handshake {
                                public_key,
                                signature,
                            },
                        ..
                    },
                ..
            }) = &ev
            {
                match self.handle_handshake(peer, public_key, signature) {
                    Ok(_) => {
                        self.pending_handshake_peers.remove(&peer);
                    }
                    Err(e) => {
                        self.handle_handshake_failure(peer, "Handshake request failed");
                    }
                }
            }

            return Poll::Ready(ev);
        }

        // Track peers whose handshake requests have timed out
        const INBOUND_HANDSHAKE_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
        let now = Instant::now();
        if let Some((&peer_id, _)) = self
            .pending_handshake_peers
            .clone()
            .into_read_only()
            .iter()
            .find(|(_, &connected_instant)| {
                now.duration_since(connected_instant) > INBOUND_HANDSHAKE_WAIT_TIMEOUT
            })
        {
            self.pending_handshake_peers.remove(&peer_id);
            self.handle_handshake_failure(&peer_id, "Handshake request timeout");
            debug!(peer=%peer_id, "Handshake request timed out, marking failure");
        }

        Poll::Pending
    }
}
