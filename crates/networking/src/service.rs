use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use crate::{
    behaviours::{GadgetBehaviour, GadgetBehaviourEvent},
    blueprint_protocol::{BlueprintProtocolEvent, InstanceMessageRequest, InstanceMessageResponse},
    discovery::{
        behaviour::{DerivedDiscoveryBehaviourEvent, DiscoveryEvent},
        PeerInfo, PeerManager,
    },
    error::Error,
    key_types::{InstanceMsgKeyPair, InstanceMsgPublicKey, InstanceSignedMsgSignature},
    service_handle::NetworkServiceHandle,
    types::ProtocolMessage,
};
use crossbeam_channel::{self, Receiver, Sender};
use dashmap::DashMap;
use futures::StreamExt;
use gadget_logging::trace;
use libp2p::{
    identify, identity::Keypair, ping, swarm::SwarmEvent, Multiaddr, PeerId, Swarm, SwarmBuilder,
    Transport,
};
use tracing::{debug, info, warn};

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

/// Network message types
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
    /// Instance id for blueprint protocol
    pub instance_id: String,
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
    network_sender: Sender<NetworkMessage>,
    /// Channel for receiving messages from the network service
    network_receiver: Receiver<NetworkMessage>,
    /// Channel for sending messages to the network service
    protocol_message_sender: Sender<ProtocolMessage>,
    /// Channel for receiving messages from the network service
    protocol_message_receiver: Receiver<ProtocolMessage>,
    /// Channel for sending events to the network service
    event_sender: Sender<NetworkEvent>,
    /// Channel for receiving events from the network service
    event_receiver: Receiver<NetworkEvent>,
    /// Network name/namespace
    network_name: String,
    /// Bootstrap peers
    bootstrap_peers: HashMap<PeerId, Multiaddr>,
}

impl NetworkService {
    /// Create a new network service
    pub async fn new(
        config: NetworkConfig,
        allowed_keys: HashSet<InstanceMsgPublicKey>,
        allowed_keys_rx: Receiver<HashSet<InstanceMsgPublicKey>>,
    ) -> Result<Self, Error> {
        let NetworkConfig {
            network_name,
            instance_id,
            instance_secret_key,
            instance_public_key,
            local_key,
            listen_addr,
            target_peer_count,
            bootstrap_peers,
            enable_mdns,
            enable_kademlia,
        } = config;

        let peer_manager = Arc::new(PeerManager::new(allowed_keys));
        let blueprint_protocol_name = format!("/blueprint_protocol/{}/1.0.0", instance_id);

        let (network_sender, network_receiver) = crossbeam_channel::unbounded();
        let (protocol_message_sender, protocol_message_receiver) = crossbeam_channel::unbounded();
        let (event_sender, event_receiver) = crossbeam_channel::unbounded();

        // Create the swarm
        let behaviour = GadgetBehaviour::new(
            &network_name,
            &blueprint_protocol_name,
            &local_key,
            &instance_secret_key,
            &instance_public_key,
            target_peer_count,
            peer_manager.clone(),
            protocol_message_sender.clone(),
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
        let bootstrap_peers = bootstrap_peers.into_iter().collect();

        Ok(Self {
            swarm,
            peer_manager,
            network_sender,
            network_receiver,
            protocol_message_sender,
            protocol_message_receiver,
            event_sender,
            event_receiver,
            network_name,
            bootstrap_peers,
        })
    }

    /// Get a sender to send messages to the network service
    pub fn network_sender(&self) -> Sender<NetworkMessage> {
        self.network_sender.clone()
    }

    pub fn start(self) -> NetworkServiceHandle {
        let local_peer_id = *self.swarm.local_peer_id();
        let public_keys_to_peer_ids = Arc::new(DashMap::new());
        let network_sender = self.network_sender.clone();
        let protocol_message_receiver = self.protocol_message_receiver.clone();

        // Create handle with new interface
        let handle = NetworkServiceHandle::new(
            local_peer_id,
            public_keys_to_peer_ids.clone(),
            network_sender,
            protocol_message_receiver,
        );

        // Spawn background task
        tokio::spawn(async move {
            self.run().await;
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
        for (peer_id, addr) in &self.bootstrap_peers {
            debug!("Dialing bootstrap peer {} at {}", peer_id, addr);
            if let Err(e) = self.swarm.dial(addr.clone()) {
                warn!("Failed to dial bootstrap peer: {}", e);
            }
        }

        loop {
            tokio::select! {
                swarm_event = self.swarm.select_next_some() => {
                    if let SwarmEvent::Behaviour(event) = swarm_event {
                        if let Err(e) = handle_behaviour_event(
                            &mut self.swarm,
                            &self.peer_manager,
                            event,
                            &self.event_sender,
                        )
                        .await
                        {
                            warn!("Failed to handle swarm event: {}", e);
                        }
                    }
                }
                Ok(msg) = async { self.network_receiver.try_recv() } => {
                    if let Err(e) = handle_network_message(
                        &mut self.swarm,
                        msg,
                        &self.peer_manager,
                        &self.event_sender,
                    )
                    .await
                    {
                        warn!("Failed to handle network message: {}", e);
                    }
                }
                else => break,
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
    event_sender: &Sender<NetworkEvent>,
) -> Result<(), Error> {
    if let SwarmEvent::Behaviour(behaviour_event) = event {
        handle_behaviour_event(swarm, peer_manager, behaviour_event, event_sender).await?;
    }

    Ok(())
}

/// Handle a behaviour event
async fn handle_behaviour_event(
    swarm: &mut Swarm<GadgetBehaviour>,
    peer_manager: &Arc<PeerManager>,
    event: GadgetBehaviourEvent,
    event_sender: &Sender<NetworkEvent>,
) -> Result<(), Error> {
    match event {
        GadgetBehaviourEvent::ConnectionLimits(_) => {}
        GadgetBehaviourEvent::Discovery(discovery_event) => {
            handle_discovery_event(
                &swarm.behaviour().discovery.peer_info,
                peer_manager,
                discovery_event,
                event_sender,
                &swarm.behaviour().blueprint_protocol.blueprint_protocol_name,
            )
            .await?;
        }
        GadgetBehaviourEvent::BlueprintProtocol(blueprint_event) => {
            handle_blueprint_protocol_event(swarm, peer_manager, blueprint_event, event_sender)
                .await?;
        }
        GadgetBehaviourEvent::Ping(ping_event) => {
            handle_ping_event(swarm, peer_manager, ping_event, event_sender).await?;
        }
    }

    Ok(())
}

/// Handle a discovery event
async fn handle_discovery_event(
    peer_info_map: &HashMap<PeerId, PeerInfo>,
    peer_manager: &Arc<PeerManager>,
    event: DiscoveryEvent,
    event_sender: &Sender<NetworkEvent>,
    blueprint_protocol_name: &str,
) -> Result<(), Error> {
    match event {
        DiscoveryEvent::PeerConnected(peer_id) => {
            trace!("Peer connected, {peer_id}");
            event_sender.send(NetworkEvent::PeerConnected(peer_id))?;
        }
        DiscoveryEvent::PeerDisconnected(peer_id) => {
            trace!("Peer disconnected, {peer_id}");
            event_sender.send(NetworkEvent::PeerDisconnected(peer_id))?;
        }
        DiscoveryEvent::Discovery(discovery_event) => match &*discovery_event {
            DerivedDiscoveryBehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
                ..
            }) => {
                let protocols: HashSet<String> =
                    HashSet::from_iter(info.protocols.iter().map(std::string::ToString::to_string));
                if !protocols.contains(blueprint_protocol_name) {
                    peer_manager
                        .ban_peer_with_default_duration(*peer_id, "hello protocol unsupported");
                }
            }
            DerivedDiscoveryBehaviourEvent::Identify(_) => {}
            _ => {}
        },
    }

    Ok(())
}

/// Handle a blueprint event
async fn handle_blueprint_protocol_event(
    swarm: &mut Swarm<GadgetBehaviour>,
    peer_manager: &Arc<PeerManager>,
    event: BlueprintProtocolEvent,
    event_sender: &Sender<NetworkEvent>,
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
    event_sender: &Sender<NetworkEvent>,
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
    event_sender: &Sender<NetworkEvent>,
) -> Result<(), Error> {
    Ok(())
}
