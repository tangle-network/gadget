use crate::util::DebugLogger;
use crate::MpEcdsaProtocolConfig;
use async_trait::async_trait;
use gadget_core::job_manager::WorkManagerInterface;
use libp2p::{
    core::upgrade,
    gossipsub, identity, mdns, noise,
    swarm::NetworkBehaviour,
    swarm::{SwarmBuilder, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Transport,
};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::time::Duration;
use webb_gadget::gadget::message::GadgetProtocolMessage;
use webb_gadget::gadget::network::Network;
use webb_gadget::gadget::work_manager::WebbWorkManager;

#[derive(Clone)]
pub struct GossipNetwork {
    tx_to_network: tokio::sync::mpsc::UnboundedSender<GadgetProtocolMessage>,
}

pub async fn create_network(
    debug_logger: DebugLogger,
    config: &MpEcdsaProtocolConfig,
) -> Result<GossipNetwork, Box<dyn Error>> {
    let bootnode = config.gossip_bootnode.clone();
    let (tx_to_network, rx_from_gadget) = tokio::sync::mpsc::unbounded_channel();

    let orientation = if let Some(bind_addr) = config.gossip_bind_addr {
        GossipNetworkOrientation::Bootnode { bind_addr }
    } else if let Some(boot_node) = config.gossip_bootnode {
        GossipNetworkOrientation::Peer { boot_node }
    } else {
        return Err("Must specify either a bootnode or a bind address".into());
    };

    let network_process = network_process(orientation, debug_logger.clone(), rx_from_gadget);

    let gossip_network = GossipNetwork { tx_to_network };

    Ok(gossip_network)
}

#[async_trait]
impl Network for GossipNetwork {
    async fn next_message(
        &self,
    ) -> Option<<WebbWorkManager as WorkManagerInterface>::ProtocolMessage> {
        todo!()
    }

    async fn send_message(
        &self,
        message: <WebbWorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), webb_gadget::Error> {
        self.tx_to_network
            .send(message)
            .map_err(|Err| webb_gadget::Error::ClientError { err: Err.into() })
    }

    async fn run(&self) -> Result<(), webb_gadget::Error> {
        todo!()
    }
}

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

enum GossipNetworkOrientation {
    Bootnode { bind_addr: SocketAddr },
    Peer { boot_node: SocketAddr },
}

async fn network_process(
    gossip_network_orientation: GossipNetworkOrientation,
    logger: DebugLogger,
    rx_from_gadget: tokio::sync::mpsc::UnboundedReceiver<GadgetProtocolMessage>,
) {
    // Create a random PeerId
    let id_keys = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(id_keys.public());
    logger.info(format!("Local peer id: {local_peer_id}"));

    // Set up an encrypted DNS-enabled TCP Transport over the Mplex protocol.
    let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(upgrade::Version::V1)
        .authenticate(
            noise::NoiseAuthenticated::xx(&id_keys).expect("signing libp2p-noise static keypair"),
        )
        .multiplex(yamux::Config::default())
        .timeout(Duration::from_secs(20))
        .boxed();

    // To content-address message, we can take the hash of message and use it as an ID.
    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    // Set a custom gossipsub configuration
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
        .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
        .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
        .build()
        .expect("Valid config");

    // build a gossipsub network behaviour
    let mut gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(id_keys),
        gossipsub_config,
    )
    .expect("Correct configuration");
    // Create a Gossipsub topic
    let topic = gossipsub::IdentTopic::new("ecdsa-network");
    // subscribes to our topic
    gossipsub.subscribe(&topic)?;

    // Create a Swarm to manage peers and events
    let mut swarm = {
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;
        let behaviour = MyBehaviour { gossipsub, mdns };
        SwarmBuilder::with_tokio_executor(tcp_transport, behaviour, local_peer_id).build()
    };

    match gossip_network_orientation {
        GossipNetworkOrientation::Bootnode { bind_addr } => {
            swarm.listen_on(
                format!("/ip4/{}/tcp/{}", bind_addr.ip(), bind_addr.port())
                    .parse::<Multiaddr>()
                    .expect("Should be valid"),
            )?;
        }
        GossipNetworkOrientation::Peer { boot_node } => {
            swarm.dial(
                format!("/ip4/{}/tcp/{}", boot_node.ip(), boot_node.port())
                    .parse::<Multiaddr>()
                    .expect("Should be valid"),
            )?;
        }
    }

    // Kick it off
    loop {
        tokio::select! {
            packet = rx_from_gadget.next() => {
                // TODO: Receive messages sent to gadget, then, either publish or send to peer directly
                if let Some(outgoing_packet) = packet {
                    if let Err(e) = swarm
                        .behaviour_mut().gossipsub
                        .publish(topic.clone(), line.expect("Stdin not to close").as_bytes()) {
                        logger.error(format!("Failed to publish message: {e}"))
                    }
                } else {
                    logger.info(format!("Receiver from gadget closed, shutting down..."));
                    return;
                }
            },
            event = swarm.select_next_some() => {
                match event {
                SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                    logger.info(format!("Connected to {peer_id} at {}", endpoint.get_remote_address()));
                }
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        logger.info(format!("mDNS discovered a new peer: {peer_id}"));
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        logger.warn(format!("mDNS discover peer has expired: {peer_id}"));
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => {
                    logger.trace(format!(
                        "Got message with id: {id} from peer: {peer_id}",
                    ));
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    logger.info(format!("Local node is listening on {address}"));
                }
                _ => {}
            }
            }
        }
    }
}
