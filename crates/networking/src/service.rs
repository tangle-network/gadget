use crate::{
    behaviours::{GadgetBehaviour, GadgetBehaviourConfig, GadgetBehaviourEvent},
    blueprint_protocol::{BlueprintProtocolEvent, InstanceMessageRequest, InstanceMessageResponse},
    discovery::{
        behaviour::{DerivedDiscoveryBehaviourEvent, DiscoveryEvent},
        PeerInfo, PeerManager,
    },
    error::Error,
    service_handle::NetworkServiceHandle,
    types::ProtocolMessage,
};
use alloy_primitives::Address;
use crossbeam_channel::{self, Receiver, SendError, Sender};
use futures::StreamExt;
use gadget_crypto::KeyType;
use libp2p::{
    identify,
    identity::Keypair,
    kad, mdns, ping,
    swarm::{dial_opts::DialOpts, SwarmEvent},
    Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use std::{collections::HashSet, fmt::Display, sync::Arc, time::Duration};
use tracing::trace;
use tracing::{debug, info, warn};

/// Events emitted by the network service
#[derive(Debug)]
pub enum NetworkEvent<K: KeyType> {
    /// New request received from a peer
    InstanceRequestInbound {
        peer: PeerId,
        request: InstanceMessageRequest<K>,
    },
    /// New response received from a peer
    InstanceResponseInbound {
        peer: PeerId,
        response: InstanceMessageResponse<K>,
    },
    /// New request sent to a peer
    InstanceRequestOutbound {
        peer: PeerId,
        request: InstanceMessageRequest<K>,
    },
    /// Response sent to a peer
    InstanceResponseOutbound {
        peer: PeerId,
        response: InstanceMessageResponse<K>,
    },
    /// New gossip message received
    GossipReceived {
        source: PeerId,
        topic: String,
        message: Vec<u8>,
    },
    /// New gossip message sent
    GossipSent { topic: String, message: Vec<u8> },
    /// Peer connected
    PeerConnected(PeerId),
    /// Peer disconnected
    PeerDisconnected(PeerId),
    /// Handshake completed successfully
    HandshakeCompleted { peer: PeerId },
    /// Handshake failed
    HandshakeFailed { peer: PeerId, reason: String },
}

#[derive(Debug)]
pub enum NetworkEventSendError<K: KeyType> {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    HandshakeCompleted {
        peer: PeerId,
    },
    HandshakeFailed {
        peer: PeerId,
        reason: String,
    },
    InstanceRequestInbound {
        peer: PeerId,
        request: InstanceMessageRequest<K>,
    },
    InstanceResponseInbound {
        peer: PeerId,
        response: InstanceMessageResponse<K>,
    },
    InstanceRequestOutbound {
        peer: PeerId,
        request: InstanceMessageRequest<K>,
    },
    InstanceResponseOutbound {
        peer: PeerId,
        response: InstanceMessageResponse<K>,
    },
    GossipReceived {
        source: PeerId,
        topic: String,
        message: Vec<u8>,
    },
    GossipSent {
        topic: String,
        message: Vec<u8>,
    },
}

impl<K: KeyType> Display for NetworkEventSendError<K> {
    fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
        match self {
            NetworkEventSendError::PeerConnected(peer) => {
                write!(f, "Error sending Peer connected event: {}", peer)
            }
            NetworkEventSendError::PeerDisconnected(peer) => {
                write!(f, "Error sending Peer disconnected event: {}", peer)
            }
            NetworkEventSendError::HandshakeCompleted { peer } => {
                write!(f, "Error sending Handshake completed event: {}", peer)
            }
            NetworkEventSendError::HandshakeFailed { peer, reason } => {
                write!(
                    f,
                    "Error sending Handshake failed event: {} ({})",
                    peer, reason
                )
            }
            NetworkEventSendError::InstanceRequestInbound { peer, request } => {
                write!(
                    f,
                    "Error sending Instance request inbound event: {} ({:#?})",
                    peer, request
                )
            }
            NetworkEventSendError::InstanceResponseInbound { peer, response } => {
                write!(
                    f,
                    "Error sending Instance response inbound event: {} ({:#?})",
                    peer, response
                )
            }
            NetworkEventSendError::InstanceRequestOutbound { peer, request } => {
                write!(
                    f,
                    "Error sending Instance request outbound event: {} ({:#?})",
                    peer, request
                )
            }
            NetworkEventSendError::InstanceResponseOutbound { peer, response } => {
                write!(
                    f,
                    "Error sending Instance response outbound event: {} ({:#?})",
                    peer, response
                )
            }
            NetworkEventSendError::GossipReceived {
                source,
                topic,
                message,
            } => {
                write!(
                    f,
                    "Error sending Gossip received event on topic: {} from source: {} ({:#?})",
                    topic, source, message
                )
            }
            NetworkEventSendError::GossipSent { topic, message } => {
                write!(
                    f,
                    "Error sending Gossip sent event on topic: {} ({:#?})",
                    topic, message
                )
            }
        }
    }
}

/// Network message types
#[derive(Debug)]
pub enum NetworkMessage<K: KeyType> {
    InstanceRequest {
        peer: PeerId,
        request: InstanceMessageRequest<K>,
    },
    GossipMessage {
        source: PeerId,
        topic: String,
        message: Vec<u8>,
    },
}

/// Configuration for the network service
#[derive(Debug, Clone)]
pub struct NetworkConfig<K: KeyType> {
    /// Network name/namespace
    pub network_name: String,
    /// Instance id for blueprint protocol
    pub instance_id: String,
    /// Instance secret key for blueprint protocol
    pub instance_key_pair: K::Secret,
    /// Local keypair for authentication
    pub local_key: Keypair,
    /// Address to listen on
    pub listen_addr: Multiaddr,
    /// Target number of peers to maintain
    pub target_peer_count: u32,
    /// Bootstrap peers to connect to
    pub bootstrap_peers: Vec<Multiaddr>,
    /// Whether to enable mDNS discovery
    pub enable_mdns: bool,
    /// Whether to enable Kademlia DHT
    pub enable_kademlia: bool,
    /// Whether to use evm addresses for verification of handshakes and msgs
    pub using_evm_address_for_handshake_verification: bool,
}

pub struct NetworkService<K: KeyType> {
    /// The libp2p swarm
    swarm: Swarm<GadgetBehaviour<K>>,
    /// Peer manager for tracking peer states
    pub(crate) peer_manager: Arc<PeerManager<K>>,
    /// Channel for sending messages to the network service
    network_sender: Sender<NetworkMessage<K>>,
    /// Channel for receiving messages from the network service
    network_receiver: Receiver<NetworkMessage<K>>,
    /// Channel for receiving messages from the network service
    protocol_message_receiver: Receiver<ProtocolMessage<K>>,
    /// Channel for sending events to the network service
    event_sender: Sender<NetworkEvent<K>>,
    /// Channel for receiving events from the network service
    #[expect(dead_code)] // For future use
    event_receiver: Receiver<NetworkEvent<K>>,
    /// Bootstrap peers
    bootstrap_peers: HashSet<Multiaddr>,
    /// Channel for receiving allowed keys updates
    allowed_keys_rx: Receiver<AllowedKeys<K>>,
}

pub enum AllowedKeys<K: KeyType> {
    EvmAddresses(gadget_std::collections::HashSet<Address>),
    InstancePublicKeys(gadget_std::collections::HashSet<K::Public>),
}

impl<K: KeyType> NetworkService<K> {
    /// Create a new network service
    ///
    /// # Errors
    ///
    /// * See [`GadgetBehaviour::new`]
    /// * Bad `listen_addr` in the provided [`NetworkConfig`]
    #[allow(clippy::missing_panics_doc)] // Unwrapping an Infallible
    pub fn new(
        config: NetworkConfig<K>,
        allowed_keys: AllowedKeys<K>,
        allowed_keys_rx: Receiver<AllowedKeys<K>>,
    ) -> Result<Self, Error> {
        let NetworkConfig::<K> {
            network_name,
            instance_id,
            instance_key_pair,
            local_key,
            listen_addr,
            target_peer_count,
            bootstrap_peers,
            enable_mdns: _,
            enable_kademlia: _,
            using_evm_address_for_handshake_verification,
            ..
        } = config;

        let peer_manager = Arc::new(PeerManager::new(allowed_keys));
        let blueprint_protocol_name = format!("/{network_name}/{instance_id}");

        let (network_sender, network_receiver) = crossbeam_channel::unbounded();
        let (protocol_message_sender, protocol_message_receiver) = crossbeam_channel::unbounded();
        let (event_sender, event_receiver) = crossbeam_channel::unbounded();

        // Create the swarm
        let gadget_behaviour_config = GadgetBehaviourConfig {
            network_name,
            blueprint_protocol_name: blueprint_protocol_name.clone(),
            local_key: local_key.clone(),
            instance_key_pair,
            target_peer_count,
            peer_manager: peer_manager.clone(),
            protocol_message_sender,
            using_evm_address_for_handshake_verification,
        };
        let behaviour = GadgetBehaviour::new(gadget_behaviour_config)?;

        let mut swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default().nodelay(true),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_quic_config(|mut config| {
                config.handshake_timeout = Duration::from_secs(30);
                config
            })
            .with_dns()?
            .with_behaviour(|_| behaviour)
            .unwrap()
            .build();

        swarm
            .behaviour_mut()
            .blueprint_protocol
            .subscribe(&blueprint_protocol_name)?;

        // Start listening
        swarm.listen_on(listen_addr)?;
        let bootstrap_peers = bootstrap_peers.into_iter().collect();

        Ok(Self {
            swarm,
            peer_manager,
            network_sender,
            network_receiver,
            protocol_message_receiver,
            event_sender,
            event_receiver,
            bootstrap_peers,
            allowed_keys_rx,
        })
    }

    /// Get a sender to send messages to the network service
    pub fn network_sender(&self) -> Sender<NetworkMessage<K>> {
        self.network_sender.clone()
    }

    pub fn start(self) -> NetworkServiceHandle<K> {
        let local_peer_id = *self.swarm.local_peer_id();
        let network_sender = self.network_sender.clone();
        let protocol_message_receiver = self.protocol_message_receiver.clone();

        // Create handle with new interface
        let handle = NetworkServiceHandle::new(
            local_peer_id,
            self.swarm
                .behaviour()
                .blueprint_protocol
                .blueprint_protocol_name
                .clone(),
            self.peer_manager.clone(),
            network_sender,
            protocol_message_receiver,
        );

        // Add our own peer ID to the peer manager with all listening addresses
        let mut info = PeerInfo::default();
        for addr in self.swarm.listeners() {
            info.addresses.insert(addr.clone());
        }
        self.peer_manager.update_peer(local_peer_id, info);

        // Start allowed keys updater
        let peer_manager = self.peer_manager.clone();
        let allowed_keys_rx = self.allowed_keys_rx.clone();
        tokio::spawn(async move {
            peer_manager.run_allowed_keys_updater(&allowed_keys_rx);
        });

        // Spawn background task
        tokio::spawn(async move {
            Box::pin(self.run()).await;
        });

        handle
    }

    /// Run the network service
    async fn run(mut self) {
        info!("Starting network service");

        // Bootstrap with Kademlia
        if let Err(e) = self.swarm.behaviour_mut().bootstrap() {
            warn!("Failed to bootstrap with Kademlia: {}", e);
        }

        // Connect to bootstrap peers
        for addr in &self.bootstrap_peers {
            debug!("Dialing bootstrap peer at {}", addr);
            if let Err(e) = self.swarm.dial(addr.clone()) {
                warn!("Failed to dial bootstrap peer: {}", e);
            }
        }

        loop {
            tokio::select! {
                swarm_event = self.swarm.select_next_some() => {
                    match swarm_event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!("New listen address: {}", address);
                            let local_peer_id = *self.swarm.local_peer_id();
                            let mut info = self.peer_manager.get_peer_info(&local_peer_id)
                                .unwrap_or_default();
                            info.addresses.insert(address.clone());
                            self.peer_manager.update_peer(local_peer_id, info);
                        },
                        SwarmEvent::Behaviour(event) => {
                            if let Err(e) = handle_behaviour_event(
                                &mut self.swarm,
                                &self.peer_manager,
                                event,
                                &self.event_sender,
                            )
                            {
                                warn!("Failed to handle swarm event: {}", e);
                            }
                        },
                        _ => {}
                    }
                }
                Ok(msg) = async { self.network_receiver.try_recv() } => {
                    if let Err(e) = handle_network_message(
                        &mut self.swarm,
                        msg,
                        &self.peer_manager,
                        &self.event_sender,
                    )
                    {
                        warn!("Failed to handle network message: {}", e);
                    }
                }
                else => break,
            }
        }

        info!("Network service stopped");
    }

    /// Get the current listening address
    pub fn get_listen_addr(&self) -> Option<Multiaddr> {
        self.swarm.listeners().next().cloned()
    }
}

/// Handle a behaviour event
fn handle_behaviour_event<K: KeyType>(
    swarm: &mut Swarm<GadgetBehaviour<K>>,
    peer_manager: &Arc<PeerManager<K>>,
    event: GadgetBehaviourEvent<K>,
    event_sender: &Sender<NetworkEvent<K>>,
) -> Result<(), Error> {
    match event {
        GadgetBehaviourEvent::ConnectionLimits(_) => {}
        GadgetBehaviourEvent::Discovery(discovery_event) => {
            handle_discovery_event(swarm, peer_manager, discovery_event, event_sender)?;
        }
        GadgetBehaviourEvent::BlueprintProtocol(blueprint_event) => {
            handle_blueprint_protocol_event(swarm, peer_manager, blueprint_event, event_sender)?;
        }
        GadgetBehaviourEvent::Ping(ping_event) => {
            handle_ping_event(swarm, peer_manager, ping_event, event_sender)?;
        }
    }

    Ok(())
}

/// Handle a discovery event
fn handle_discovery_event<K: KeyType>(
    swarm: &mut Swarm<GadgetBehaviour<K>>,
    peer_manager: &Arc<PeerManager<K>>,
    event: DiscoveryEvent,
    event_sender: &Sender<NetworkEvent<K>>,
) -> Result<(), Error> {
    match event {
        DiscoveryEvent::PeerConnected(peer_id) => {
            info!("Peer connected, {peer_id}");
            // Update peer info when connected
            if let Some(info) = swarm.behaviour().discovery.peer_info.get(&peer_id) {
                peer_manager.update_peer(peer_id, info.clone());
            }
            event_sender
                .send(NetworkEvent::PeerConnected(peer_id))
                .map_err(|_| {
                    SendError(NetworkEventSendError::<K>::PeerConnected(peer_id).to_string())
                })?;
        }
        DiscoveryEvent::PeerDisconnected(peer_id) => {
            info!("Peer disconnected, {peer_id}");
            peer_manager.remove_peer(&peer_id, "disconnected");
            event_sender
                .send(NetworkEvent::PeerDisconnected(peer_id))
                .map_err(|_| {
                    SendError(NetworkEventSendError::<K>::PeerDisconnected(peer_id).to_string())
                })?;
        }
        DiscoveryEvent::Discovery(discovery_event) => match &*discovery_event {
            DerivedDiscoveryBehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
                ..
            }) => {
                info!(%peer_id, "Received identify event");
                let protocols: HashSet<String> = info
                    .protocols
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect();

                trace!(%peer_id, ?protocols, "Supported protocols");

                let blueprint_protocol_name =
                    &swarm.behaviour().blueprint_protocol.blueprint_protocol_name;
                if !protocols.contains(blueprint_protocol_name) {
                    warn!(%peer_id, %blueprint_protocol_name, "Peer does not support required protocol");
                    peer_manager.ban_peer_with_default_duration(*peer_id, "protocol unsupported");
                    return Ok(());
                }

                // Get existing peer info or create new one
                let mut peer_info = peer_manager.get_peer_info(peer_id).unwrap_or_default();

                // Update identify info
                peer_info.identify_info = Some(info.clone());

                trace!(%peer_id, listen_addrs=?info.listen_addrs, "Adding identify addresses");
                // Add all addresses from identify info
                for addr in &info.listen_addrs {
                    peer_info.addresses.insert(addr.clone());
                }

                trace!(%peer_id, "Updating peer info with identify information");
                peer_manager.update_peer(*peer_id, peer_info);
                debug!(%peer_id, "Successfully processed identify information");
            }
            DerivedDiscoveryBehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed {
                result: kad::QueryResult::GetClosestPeers(Ok(ok)),
                ..
            }) => {
                // Process newly discovered peers
                for peer_info in &ok.peers {
                    if !peer_manager.get_peers().contains_key(&peer_info.peer_id) {
                        info!(%peer_info.peer_id, "Newly discovered peer from Kademlia");
                        let info = PeerInfo::default();
                        peer_manager.update_peer(peer_info.peer_id, info);
                        let addrs: Vec<_> = peer_info.addrs.clone();
                        for addr in addrs {
                            debug!(%peer_info.peer_id, %addr, "Dialing peer from Kademlia");
                            if let Err(e) = swarm.dial(DialOpts::from(addr)) {
                                warn!("Failed to dial address: {}", e);
                            }
                        }
                    }
                }
            }
            DerivedDiscoveryBehaviourEvent::Mdns(mdns::Event::Discovered(list)) => {
                // Add newly discovered peers from mDNS
                for (peer_id, addr) in list {
                    if !peer_manager.get_peers().contains_key(peer_id) {
                        info!(%peer_id, %addr, "Newly discovered peer from Mdns");
                        let mut info = PeerInfo::default();
                        info.addresses.insert(addr.clone());
                        peer_manager.update_peer(*peer_id, info);
                        debug!(%peer_id, %addr, "Dialing peer from Mdns");
                        if let Err(e) = swarm.dial(DialOpts::from(addr.clone())) {
                            warn!("Failed to dial address: {}", e);
                        }
                    }
                }
            }
            _ => {}
        },
    }

    Ok(())
}

/// Handle a blueprint event
fn handle_blueprint_protocol_event<K: KeyType>(
    _swarm: &mut Swarm<GadgetBehaviour<K>>,
    _peer_manager: &Arc<PeerManager<K>>,
    event: BlueprintProtocolEvent<K>,
    event_sender: &Sender<NetworkEvent<K>>,
) -> Result<(), Error> {
    match event {
        BlueprintProtocolEvent::Request {
            peer,
            request,
            channel: _,
        } => event_sender
            .send(NetworkEvent::InstanceRequestInbound {
                peer,
                request: request.clone(),
            })
            .map_err(|_| {
                SendError(
                    NetworkEventSendError::<K>::InstanceRequestInbound { peer, request }
                        .to_string(),
                )
            })?,
        BlueprintProtocolEvent::Response {
            peer,
            response,
            request_id: _,
        } => event_sender
            .send(NetworkEvent::InstanceResponseInbound {
                peer,
                response: response.clone(),
            })
            .map_err(|_| {
                SendError(
                    NetworkEventSendError::<K>::InstanceResponseInbound { peer, response }
                        .to_string(),
                )
            })?,
        BlueprintProtocolEvent::GossipMessage {
            source,
            topic,
            message,
        } => event_sender
            .send(NetworkEvent::GossipReceived {
                source,
                topic: topic.to_string(),
                message: message.clone(),
            })
            .map_err(|_| {
                SendError(
                    NetworkEventSendError::<K>::GossipReceived {
                        source,
                        topic: topic.to_string(),
                        message,
                    }
                    .to_string(),
                )
            })?,
    }

    Ok(())
}

/// Handle a ping event
#[expect(clippy::unnecessary_wraps)]
fn handle_ping_event<K: KeyType>(
    _swarm: &mut Swarm<GadgetBehaviour<K>>,
    _peer_manager: &Arc<PeerManager<K>>,
    event: ping::Event,
    _event_sender: &Sender<NetworkEvent<K>>,
) -> Result<(), Error> {
    match event.result {
        Ok(rtt) => {
            trace!(
                "PingSuccess::Ping rtt to {} is {} ms",
                event.peer,
                rtt.as_millis()
            );
        }
        Err(ping::Failure::Unsupported) => {
            debug!(peer=%event.peer, "Ping protocol unsupported");
        }
        Err(ping::Failure::Timeout) => {
            debug!("Ping timeout: {}", event.peer);
        }
        Err(ping::Failure::Other { error }) => {
            debug!("Ping failure: {error}");
        }
    }

    Ok(())
}

/// Handle a network message
fn handle_network_message<K: KeyType>(
    swarm: &mut Swarm<GadgetBehaviour<K>>,
    msg: NetworkMessage<K>,
    peer_manager: &Arc<PeerManager<K>>,
    event_sender: &Sender<NetworkEvent<K>>,
) -> Result<(), Error> {
    match msg {
        NetworkMessage::InstanceRequest { peer, request } => {
            // Only send requests to verified peers
            if !peer_manager.is_peer_verified(&peer) {
                warn!(%peer, "Attempted to send request to unverified peer");
                return Ok(());
            }

            debug!(%peer, ?request, "Sending instance request");
            swarm
                .behaviour_mut()
                .blueprint_protocol
                .send_request(&peer, request.clone());
            event_sender
                .send(NetworkEvent::InstanceRequestOutbound {
                    peer,
                    request: request.clone(),
                })
                .map_err(|_| {
                    SendError(
                        NetworkEventSendError::<K>::InstanceRequestOutbound { peer, request }
                            .to_string(),
                    )
                })?;
        }
        NetworkMessage::GossipMessage {
            source,
            topic,
            message,
        } => {
            debug!(%source, %topic, "Publishing gossip message");
            if let Err(e) = swarm
                .behaviour_mut()
                .blueprint_protocol
                .publish(&topic, message.clone())
            {
                warn!(%source, %topic, "Failed to publish gossip message: {:?}", e);
                return Ok(());
            }
            event_sender
                .send(NetworkEvent::GossipSent {
                    topic: topic.to_string(),
                    message: message.clone(),
                })
                .map_err(|_| {
                    SendError(NetworkEventSendError::<K>::GossipSent { topic, message }.to_string())
                })?;
        }
    }

    Ok(())
}
