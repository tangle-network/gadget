use super::{InstanceMessageRequest, InstanceMessageResponse};
use crate::blueprint_protocol::HandshakeMessage;
use crate::discovery::peers::VerificationIdentifierKey;
use crate::discovery::utils::get_address_from_compressed_pubkey;
use crate::discovery::PeerManager;
use crate::types::ProtocolMessage;
use bincode;
use crossbeam_channel::Sender;
use dashmap::DashMap;
use gadget_crypto::BytesEncoding;
use gadget_crypto::KeyType;
use libp2p::{
    core::transport::PortUse,
    gossipsub::{self, IdentTopic, MessageId, Sha256Topic},
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
use tracing::{debug, error, info, trace, warn};

#[derive(NetworkBehaviour)]
pub struct DerivedBlueprintProtocolBehaviour<K: KeyType> {
    /// Request/response protocol for p2p messaging
    request_response:
        request_response::cbor::Behaviour<InstanceMessageRequest<K>, InstanceMessageResponse<K>>,
    /// Gossipsub for broadcast messaging
    gossipsub: gossipsub::Behaviour,
}

/// Events emitted by the `BlueprintProtocolBehaviour`
#[derive(Debug)]
pub enum BlueprintProtocolEvent<K: KeyType> {
    /// Request received from a peer
    Request {
        peer: PeerId,
        request: InstanceMessageRequest<K>,
        channel: ResponseChannel<InstanceMessageResponse<K>>,
    },
    /// Response received from a peer
    Response {
        peer: PeerId,
        request_id: OutboundRequestId,
        response: InstanceMessageResponse<K>,
    },
    /// Gossip message received
    GossipMessage {
        source: PeerId,
        message: Vec<u8>,
        topic: IdentTopic,
    },
}

/// Behaviour that handles the blueprint protocol request/response and gossip
pub struct BlueprintProtocolBehaviour<K: KeyType> {
    /// Request/response protocol for direct messaging
    blueprint_protocol: DerivedBlueprintProtocolBehaviour<K>,
    /// Name of the blueprint protocol
    pub(crate) blueprint_protocol_name: String,
    /// Peer manager for tracking peer states
    pub(crate) peer_manager: Arc<PeerManager<K>>,
    /// Libp2p peer ID
    pub(crate) local_peer_id: PeerId,
    /// Instance key pair for handshakes and blueprint protocol
    pub(crate) instance_key_pair: K::Secret,
    /// Peers with pending inbound handshakes
    pub(crate) inbound_handshakes: DashMap<PeerId, Instant>,
    /// Peers with pending outbound handshakes
    pub(crate) outbound_handshakes: DashMap<PeerId, Instant>,
    /// Active response channels
    #[expect(dead_code)] // TODO
    pub(crate) response_channels:
        DashMap<OutboundRequestId, ResponseChannel<InstanceMessageResponse<K>>>,
    /// Protocol message sender
    pub(crate) protocol_message_sender: Sender<ProtocolMessage<K>>,
    /// Flag for using addresses for whitelisting and handshake verification
    pub(crate) use_address_for_handshake_verification: bool,
}

impl<K: KeyType> BlueprintProtocolBehaviour<K> {
    /// Create a new blueprint protocol behaviour
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // Known good gossipsub config
    pub fn new(
        local_key: &Keypair,
        instance_key_pair: &K::Secret,
        peer_manager: Arc<PeerManager<K>>,
        blueprint_protocol_name: &str,
        protocol_message_sender: Sender<ProtocolMessage<K>>,
        use_address_for_handshake_verification: bool,
    ) -> Self {
        let blueprint_protocol_name = blueprint_protocol_name.to_string();
        let protocols = vec![(
            StreamProtocol::try_from_owned(blueprint_protocol_name.to_string())
                .unwrap_or_else(|_| StreamProtocol::new("/blueprint_protocol/1.0.0")),
            request_response::ProtocolSupport::Full,
        )];

        // Initialize gossipsub with message signing
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .mesh_n_low(2)
            .mesh_n(4)
            .mesh_n_high(8)
            .gossip_lazy(3)
            .history_length(10)
            .history_gossip(3)
            .flood_publish(true)
            .build()
            .expect("Valid gossipsub config");

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
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
            blueprint_protocol_name,
            peer_manager,
            local_peer_id,
            instance_key_pair: instance_key_pair.clone(),
            inbound_handshakes: DashMap::new(),
            outbound_handshakes: DashMap::new(),
            response_channels: DashMap::new(),
            protocol_message_sender,
            use_address_for_handshake_verification,
        }
    }

    /// Sign a handshake message for a peer
    #[allow(clippy::unused_self)]
    pub(crate) fn sign_handshake(
        &self,
        key_pair: &mut K::Secret,
        peer: &PeerId,
        handshake_msg: &HandshakeMessage,
    ) -> Option<K::Signature> {
        let msg = handshake_msg.to_bytes(peer);
        match K::sign_with_secret(key_pair, &msg) {
            Ok(signature) => {
                let public_key = K::public_from_secret(key_pair);
                let hex_msg = hex::encode(msg);

                debug!(%peer, ?hex_msg, ?public_key, ?signature, "signing handshake");
                Some(signature)
            }
            Err(e) => {
                warn!("Failed to sign handshake message: {e:?}");
                None
            }
        }
    }

    /// Send a request to a peer
    pub fn send_request(
        &mut self,
        peer: &PeerId,
        request: InstanceMessageRequest<K>,
    ) -> OutboundRequestId {
        debug!(%peer, ?request, "sending request");
        self.blueprint_protocol
            .request_response
            .send_request(peer, request)
    }

    /// Send a response through a response channel
    ///
    /// # Errors
    ///
    /// See [`libp2p::request_response::Behaviour::send_response`]
    pub fn send_response(
        &mut self,
        channel: ResponseChannel<InstanceMessageResponse<K>>,
        response: InstanceMessageResponse<K>,
    ) -> Result<(), InstanceMessageResponse<K>> {
        debug!(?response, "sending response");
        self.blueprint_protocol
            .request_response
            .send_response(channel, response)
    }

    /// Subscribe to a gossip topic
    ///
    /// # Errors
    ///
    /// See [`libp2p::gossipsub::SubscriptionError`]
    pub fn subscribe(&mut self, topic: &str) -> Result<bool, gossipsub::SubscriptionError> {
        let topic = Sha256Topic::new(topic);
        self.blueprint_protocol.gossipsub.subscribe(&topic)
    }

    /// Publish a message to a gossip topic
    ///
    /// # Errors
    ///
    /// See [`libp2p::gossipsub::PublishError`]
    pub fn publish(
        &mut self,
        topic: &str,
        data: impl Into<Vec<u8>>,
    ) -> Result<MessageId, gossipsub::PublishError> {
        let topic = Sha256Topic::new(topic);
        self.blueprint_protocol.gossipsub.publish(topic, data)
    }

    /// Verify and handle a handshake with a peer
    ///
    /// # Errors
    ///
    /// The handshake expired or has an invalid signature
    pub fn verify_handshake(
        &self,
        msg: &HandshakeMessage,
        verification_id_key: &VerificationIdentifierKey<K>,
        signature: &K::Signature,
    ) -> Result<(), InstanceMessageResponse<K>> {
        if msg.is_expired(HandshakeMessage::MAX_AGE) {
            error!(%msg.sender, "Handshake message expired");
            return Err(InstanceMessageResponse::Error {
                code: 400,
                message: "Handshake message expired".to_string(),
            });
        }

        let msg_bytes = msg.to_bytes(&self.local_peer_id);
        let hex_msg = hex::encode(msg_bytes.clone());

        debug!(%hex_msg, ?verification_id_key, ?signature, "verifying handshake");

        let valid = verification_id_key
            .verify(&msg_bytes, signature.to_bytes().as_ref())
            .map_err(|e| InstanceMessageResponse::Error {
                code: 400,
                message: format!("Invalid handshake signature: {e}"),
            })?;

        if !valid {
            warn!(%msg.sender, "Invalid handshake signature for peer");
            return Err(InstanceMessageResponse::Error {
                code: 400,
                message: "Invalid handshake signature".to_string(),
            });
        }

        trace!(%msg.sender, "Handshake signature verified successfully");
        Ok(())
    }

    /// Handle a handshake message, verifying the peer on success
    ///
    /// # Errors
    ///
    /// See [`Self::verify_handshake()`]
    pub fn handle_handshake(
        &self,
        msg: &HandshakeMessage,
        verification_id_key: &VerificationIdentifierKey<K>,
        signature: &K::Signature,
    ) -> Result<(), InstanceMessageResponse<K>> {
        self.verify_handshake(msg, verification_id_key, signature)?;
        self.peer_manager
            .link_peer_id_to_verification_id_key(&msg.sender, verification_id_key);

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

    pub fn handle_gossipsub_event(&mut self, event: gossipsub::Event) {
        match event {
            gossipsub::Event::Message {
                propagation_source,
                message_id: _,
                message,
            } => {
                // Only accept gossip from verified peers
                if !self.peer_manager.is_peer_verified(&propagation_source) {
                    warn!(%propagation_source, "Received gossip from unverified peer");
                    return;
                }

                debug!(%propagation_source, "Received gossip message");

                // Deserialize the protocol message
                let Ok(protocol_message) =
                    bincode::deserialize::<ProtocolMessage<K>>(&message.data)
                else {
                    warn!(%propagation_source, "Failed to deserialize gossip message");
                    return;
                };

                debug!(%propagation_source, %protocol_message, "Forwarding gossip message to protocol handler");
                if let Err(e) = self.protocol_message_sender.send(protocol_message) {
                    warn!(%propagation_source, "Failed to forward gossip message: {e}");
                }
            }
            gossipsub::Event::Subscribed { peer_id, topic } => {
                debug!(%peer_id, %topic, "Peer subscribed to topic");
            }
            gossipsub::Event::Unsubscribed { peer_id, topic } => {
                debug!(%peer_id, %topic, "Peer unsubscribed from topic");
            }
            _ => {}
        }
    }
}

impl<K: KeyType> NetworkBehaviour for BlueprintProtocolBehaviour<K> {
    type ConnectionHandler =
        <DerivedBlueprintProtocolBehaviour<K> as NetworkBehaviour>::ConnectionHandler;

    type ToSwarm = BlueprintProtocolEvent<K>;

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
            .on_connection_handler_event(peer_id, connection_id, event);
    }

    fn on_swarm_event(&mut self, event: FromSwarm<'_>) {
        match &event {
            FromSwarm::ConnectionEstablished(e) if e.other_established == 0 => {
                // Start handshake if this peer is not verified
                if !self.peer_manager.is_peer_verified(&e.peer_id) {
                    debug!(
                        "Established connection with unverified peer {:?}, sending handshake",
                        e.peer_id
                    );
                    let mut key_pair = self.instance_key_pair.clone();

                    let handshake_msg = HandshakeMessage::new(self.local_peer_id);
                    let Some(signature) =
                        self.sign_handshake(&mut key_pair, &e.peer_id, &handshake_msg)
                    else {
                        return;
                    };

                    let public_key = K::public_from_secret(&key_pair);
                    self.send_request(
                        &e.peer_id,
                        InstanceMessageRequest::Handshake {
                            verification_id_key: if self.use_address_for_handshake_verification {
                                VerificationIdentifierKey::EvmAddress(
                                    get_address_from_compressed_pubkey(public_key.to_bytes()),
                                )
                            } else {
                                VerificationIdentifierKey::InstancePublicKey(public_key)
                            },
                            signature,
                            msg: handshake_msg,
                        },
                    );
                    self.outbound_handshakes.insert(e.peer_id, Instant::now());
                    info!(
                        "Established connection to {:?}, sending handshake",
                        e.peer_id
                    );
                }

                self.blueprint_protocol
                    .gossipsub
                    .add_explicit_peer(&e.peer_id);
            }
            FromSwarm::ConnectionClosed(e) if e.remaining_established == 0 => {
                if self.inbound_handshakes.contains_key(&e.peer_id) {
                    self.inbound_handshakes.remove(&e.peer_id);
                }

                if self.outbound_handshakes.contains_key(&e.peer_id) {
                    self.outbound_handshakes.remove(&e.peer_id);
                }

                if self.peer_manager.is_peer_verified(&e.peer_id) {
                    self.peer_manager
                        .remove_peer(&e.peer_id, "connection closed");
                }

                self.blueprint_protocol
                    .gossipsub
                    .remove_explicit_peer(&e.peer_id);

                self.peer_manager
                    .remove_peer_id_from_verification_id_key(&e.peer_id);
            }

            _ => {}
        }

        self.blueprint_protocol.on_swarm_event(event);
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
                        self.handle_gossipsub_event(gossip_event);
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
