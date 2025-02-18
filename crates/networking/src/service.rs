use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    behaviours::{GadgetBehaviour, GadgetBehaviourEvent},
    blueprint_protocol::{BlueprintProtocolEvent, InstanceMessageRequest, InstanceMessageResponse},
    discovery::{
        behaviour::{DerivedDiscoveryBehaviour, DerivedDiscoveryBehaviourEvent, DiscoveryEvent},
        PeerManager,
    },
    error::Error,
    key_types::{InstanceMsgKeyPair, InstanceMsgPublicKey, InstanceSignedMsgSignature},
};
use futures::{Stream, StreamExt};
use gadget_logging::trace;
use libp2p::{
    core::{transport::Boxed, upgrade},
    gossipsub::{IdentTopic, Topic},
    identity::Keypair,
    noise, ping,
    swarm::{ConnectionId, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder, Transport,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::{IntervalStream, UnboundedReceiverStream};
use tracing::{debug, error, info, warn};

/// Events emitted by the network service
#[derive(Debug)]
pub enum NetworkEvent {
    /// New request received from a peer
    InstanceRequestInbound {
        peer: PeerId,
        request: InstanceMessageRequest,
    },
    /// New response received from a peer
    InstanceResponseInbound {
        peer: PeerId,
        response: InstanceMessageResponse,
    },
    /// New request sent to a peer
    InstanceRequestOutbound {
        peer: PeerId,
        request: InstanceMessageRequest,
    },
    /// Response sent to a peer
    InstanceResponseOutbound {
        peer: PeerId,
        response: InstanceMessageResponse,
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
pub enum NetworkMessage {
    InstanceRequest {
        peer: PeerId,
        request: InstanceMessageRequest,
    },
    InstanceResponse {
        peer: PeerId,
        response: InstanceMessageResponse,
    },
    GossipMessage {
        source: PeerId,
        topic: String,
        message: Vec<u8>,
    },
    HandshakeRequest {
        peer: PeerId,
        public_key: InstanceMsgPublicKey,
        signature: InstanceSignedMsgSignature,
    },
    HandshakeResponse {
        peer: PeerId,
        public_key: InstanceMsgPublicKey,
        signature: InstanceSignedMsgSignature,
    },
}

/// Configuration for the network service
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Network name/namespace
    pub network_name: String,
    /// Instance secret key for blueprint protocol
    pub instance_secret_key: InstanceMsgKeyPair,
    /// Instance public key for blueprint protocol
    pub instance_public_key: InstanceMsgPublicKey,
    /// Local keypair for authentication
    pub local_key: Keypair,
    /// Address to listen on
    pub listen_addr: Multiaddr,
    /// Target number of peers to maintain
    pub target_peer_count: u64,
    /// Bootstrap peers to connect to
    pub bootstrap_peers: Vec<(PeerId, Multiaddr)>,
    /// Whether to enable mDNS discovery
    pub enable_mdns: bool,
    /// Whether to enable Kademlia DHT
    pub enable_kademlia: bool,
}

pub struct NetworkService {
    /// The libp2p swarm
    swarm: Swarm<GadgetBehaviour>,
    /// Peer manager for tracking peer states
    peer_manager: Arc<PeerManager>,
    /// Channel for sending messages to the network service
    network_sender: mpsc::UnboundedSender<NetworkMessage>,
    /// Channel for receiving messages from the network service
    network_receiver: UnboundedReceiverStream<NetworkMessage>,
    /// Channel for sending events to the network service
    event_sender: mpsc::UnboundedSender<NetworkEvent>,
    /// Channel for receiving events from the network service
    event_receiver: UnboundedReceiverStream<NetworkEvent>,
    /// Network name/namespace
    network_name: String,
    /// Bootstrap peers
    bootstrap_peers: HashMap<PeerId, Multiaddr>,
}

impl NetworkService {
    /// Create a new network service
    pub async fn new(config: NetworkConfig) -> Result<Self, Error> {
        let NetworkConfig {
            network_name,
            instance_secret_key,
            instance_public_key,
            local_key,
            listen_addr,
            target_peer_count,
            bootstrap_peers,
            enable_mdns,
            enable_kademlia,
        } = config;

        let peer_manager = Arc::new(PeerManager::default());

        // Create the swarm
        let behaviour = GadgetBehaviour::new(
            &network_name,
            &local_key,
            &instance_secret_key,
            &instance_public_key,
            target_peer_count,
            peer_manager.clone(),
        );

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

        // Start listening
        swarm.listen_on(listen_addr)?;

        let (network_sender, network_receiver) = mpsc::unbounded_channel();
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let bootstrap_peers = bootstrap_peers.into_iter().collect();

        let service = Self {
            swarm,
            peer_manager,
            network_sender,
            network_receiver: UnboundedReceiverStream::new(network_receiver),
            event_sender: event_sender.clone(),
            event_receiver: UnboundedReceiverStream::new(event_receiver),
            network_name,
            bootstrap_peers,
        };

        Ok(service)
    }

    /// Get a sender to send messages to the network service
    pub fn message_sender(&self) -> mpsc::UnboundedSender<NetworkMessage> {
        self.network_sender.clone()
    }

    /// Run the network service
    async fn run(mut self, event_sender: mpsc::UnboundedSender<NetworkEvent>) {
        info!("Starting network service");

        // Bootstrap with Kademlia
        if let Err(e) = self.swarm.behaviour_mut().bootstrap() {
            warn!("Failed to bootstrap with Kademlia: {}", e);
        }

        // Connect to bootstrap peers
        for (peer_id, addr) in &self.bootstrap_peers {
            debug!("Dialing bootstrap peer {} at {}", peer_id, addr);
            if let Err(e) = self.swarm.dial(addr.clone()) {
                warn!("Failed to dial bootstrap peer: {}", e);
            }
        }

        let mut swarm_stream = self.swarm.fuse();
        let mut network_stream = self.network_receiver.fuse();
        loop {
            tokio::select! {
                message = network_stream.next() => {
                    match message {
                        Some(msg) => match handle_network_message(
                            swarm_stream.get_mut(),
                            msg,
                            &self.peer_manager,
                            &self.event_sender,
                        )
                        .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                warn!("Failed to handle network message: {}", e);
                            }
                        },
                        None => break,
                    }
                }
                swarm_event = swarm_stream.next() => match swarm_event {
                    // outbound events
                    Some(SwarmEvent::Behaviour(event)) => {
                        handle_behaviour_event(
                            swarm_stream.get_mut(),
                            &self.peer_manager,
                            event,
                            &self.event_sender,
                            &self.network_sender,
                        )
                        .await;
                    },
                    None => { break; },
                    _ => { },
                },
            }
        }

        info!("Network service stopped");
    }
}

/// Handle a swarm event
async fn handle_swarm_event(
    swarm: &mut Swarm<GadgetBehaviour>,
    peer_manager: &Arc<PeerManager>,
    event: SwarmEvent<GadgetBehaviourEvent>,
    event_sender: &mpsc::UnboundedSender<NetworkEvent>,
    network_sender: &mpsc::UnboundedSender<NetworkMessage>,
) -> Result<(), Error> {
    match event {
        SwarmEvent::Behaviour(behaviour_event) => {
            handle_behaviour_event(
                swarm,
                peer_manager,
                behaviour_event,
                event_sender,
                network_sender,
            )
            .await?
        }
        _ => {}
    }

    Ok(())
}

/// Handle a behaviour event
async fn handle_behaviour_event(
    swarm: &mut Swarm<GadgetBehaviour>,
    peer_manager: &Arc<PeerManager>,
    event: GadgetBehaviourEvent,
    event_sender: &mpsc::UnboundedSender<NetworkEvent>,
    network_sender: &mpsc::UnboundedSender<NetworkMessage>,
) -> Result<(), Error> {
    match event {
        GadgetBehaviourEvent::ConnectionLimits(_) => {}
        GadgetBehaviourEvent::Discovery(discovery_event) => match discovery_event {
            DiscoveryEvent::Discovery(derived_event) => {
                handle_discovery_event(
                    swarm,
                    peer_manager,
                    derived_event,
                    event_sender,
                    network_sender,
                )
                .await?
            }
            DiscoveryEvent::PeerConnected(peer) => {
                event_sender.send(NetworkEvent::PeerConnected(peer))?
            }
            DiscoveryEvent::PeerDisconnected(peer) => {
                event_sender.send(NetworkEvent::PeerDisconnected(peer))?
            }
        },
        GadgetBehaviourEvent::BlueprintProtocol(blueprint_event) => {
            handle_blueprint_protocol_event(
                swarm,
                peer_manager,
                blueprint_event,
                event_sender,
                network_sender,
            )
            .await?
        }
        GadgetBehaviourEvent::Ping(ping_event) => {
            handle_ping_event(
                swarm,
                peer_manager,
                ping_event,
                event_sender,
                network_sender,
            )
            .await?
        }
    }

    Ok(())
}

/// Handle a discovery event
async fn handle_discovery_event(
    swarm: &mut Swarm<GadgetBehaviour>,
    peer_manager: &Arc<PeerManager>,
    event: Box<DerivedDiscoveryBehaviourEvent>,
    event_sender: &mpsc::UnboundedSender<NetworkEvent>,
    network_sender: &mpsc::UnboundedSender<NetworkMessage>,
) -> Result<(), Error> {
    let event = event.as_ref();
    match event {
        DerivedDiscoveryBehaviourEvent::Kademlia(event) => {
            // Handle Kademlia discovery events
        }
        DerivedDiscoveryBehaviourEvent::Mdns(event) => {
            // Handle mDNS discovery events
        }
        DerivedDiscoveryBehaviourEvent::Identify(event) => {
            // Handle Identify protocol events
        }
        DerivedDiscoveryBehaviourEvent::Autonat(event) => {
            // Handle AutoNAT events
        }
        DerivedDiscoveryBehaviourEvent::Upnp(event) => {
            // Handle UPnP events
        }
        DerivedDiscoveryBehaviourEvent::Relay(event) => {
            // Handle relay events
        }
    }

    Ok(())
}

/// Handle a blueprint event
async fn handle_blueprint_protocol_event(
    swarm: &mut Swarm<GadgetBehaviour>,
    peer_manager: &Arc<PeerManager>,
    event: BlueprintProtocolEvent,
    event_sender: &mpsc::UnboundedSender<NetworkEvent>,
    network_sender: &mpsc::UnboundedSender<NetworkMessage>,
) -> Result<(), Error> {
    match event {
        BlueprintProtocolEvent::Request {
            peer,
            request,
            channel,
        } => event_sender.send(NetworkEvent::InstanceRequestInbound { peer, request })?,
        BlueprintProtocolEvent::Response {
            peer,
            response,
            request_id,
        } => event_sender.send(NetworkEvent::InstanceResponseInbound { peer, response })?,
        BlueprintProtocolEvent::GossipMessage {
            source,
            topic,
            message,
        } => event_sender.send(NetworkEvent::GossipReceived {
            source,
            topic: topic.to_string(),
            message,
        })?,
    }

    Ok(())
}

/// Handle a ping event
async fn handle_ping_event(
    swarm: &mut Swarm<GadgetBehaviour>,
    peer_manager: &Arc<PeerManager>,
    event: ping::Event,
    event_sender: &mpsc::UnboundedSender<NetworkEvent>,
    network_sender: &mpsc::UnboundedSender<NetworkMessage>,
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
async fn handle_network_message(
    swarm: &mut Swarm<GadgetBehaviour>,
    msg: NetworkMessage,
    peer_manager: &Arc<PeerManager>,
    event_sender: &mpsc::UnboundedSender<NetworkEvent>,
) -> Result<(), Error> {
    match msg {
        NetworkMessage::InstanceRequest { peer, request } => {
            event_sender.send(NetworkEvent::InstanceRequestOutbound { peer, request })?
        }
        NetworkMessage::InstanceResponse { peer, response } => {
            event_sender.send(NetworkEvent::InstanceResponseOutbound { peer, response })?
        }
        NetworkMessage::GossipMessage {
            source,
            topic,
            message,
        } => event_sender.send(NetworkEvent::GossipSent { topic, message })?,
        NetworkMessage::HandshakeRequest {
            peer,
            public_key,
            signature,
        } => event_sender.send(NetworkEvent::HandshakeCompleted { peer })?,
        NetworkMessage::HandshakeResponse {
            peer,
            public_key,
            signature,
        } => event_sender.send(NetworkEvent::HandshakeCompleted { peer })?,
    }

    Ok(())
}
