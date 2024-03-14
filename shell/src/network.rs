use crate::config::ShellConfig;
use async_trait::async_trait;
use futures::stream::StreamExt;
use gadget_common::prelude::{DebugLogger, Network, WorkManager};
use gadget_core::job_manager::WorkManagerInterface;
use libp2p::gossipsub::IdentTopic;
use libp2p::{
    gossipsub, mdns, request_response, swarm::NetworkBehaviour, swarm::SwarmEvent, PeerId,
    StreamProtocol,
};
use serde::{Deserialize, Serialize};
use sp_core::ecdsa;
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::{io, select};

// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    p2p: request_response::cbor::Behaviour<GossipMessage, GossipMessage>,
}

pub async fn setup_network(
    identity: libp2p::identity::Keypair,
    config: &ShellConfig,
    logger: DebugLogger,
    networks: Vec<&'static str>,
) -> Result<(Vec<GossipHandle>, JoinHandle<()>), Box<dyn Error>> {
    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(identity)
        .with_tokio()
        /*.with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?*/
        .with_quic()
        .with_behaviour(|key| {
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
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?; // Temporary hack because `build` does not return a proper `std::error::Error`.

            // Setup gossipsub network behaviour for broadcasting
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            // Setup mDNS for peer discovery
            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;

            // Setup request-response for direct messaging
            let p2p_config = request_response::Config::default();
            // StreamProtocols MUST begin with a forward slash
            let protocols = networks
                .iter()
                .map(|n| {
                    (
                        StreamProtocol::new(n),
                        request_response::ProtocolSupport::Full,
                    )
                })
                .collect::<Vec<_>>();

            let p2p = request_response::Behaviour::new(protocols, p2p_config);

            Ok(MyBehaviour {
                gossipsub,
                mdns,
                p2p,
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Subscribe to all networks
    let mut inbound_mapping = Vec::new();
    let (tx_to_outbound, mut rx_to_outbound) =
        tokio::sync::mpsc::unbounded_channel::<IntraNodePayload>();
    let ecdsa_peer_id_to_libp2p_id =
        Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::<
            ecdsa::Public,
            PeerId,
        >::new()));
    let mut handles_ret = vec![];

    for network in networks {
        let topic = IdentTopic::new(network);
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        let (inbound_tx, inbound_rx) = tokio::sync::mpsc::unbounded_channel();
        let connected_peers = Arc::new(AtomicU32::new(0));
        inbound_mapping.push((topic.clone(), inbound_tx, connected_peers.clone()));

        handles_ret.push(GossipHandle {
            connected_peers,
            topic,
            tx_to_outbound: tx_to_outbound.clone(),
            rx_from_inbound: Arc::new(Mutex::new(inbound_rx)),
            logger: logger.clone(),
            ecdsa_peer_id_to_libp2p_id: ecdsa_peer_id_to_libp2p_id.clone(),
        })
    }

    swarm
        .listen_on(format!("/ip4/{}/udp/{}/quic-v1", config.bind_ip, config.bind_port).parse()?)?;
    // swarm.listen_on(format!("/ip4/{}/tcp/{}", config.bind_ip, config.bind_port).parse()?)?;

    // Dial all bootnodes
    for bootnode in &config.bootnodes {
        swarm.dial(bootnode.clone())?;
    }

    let worker = async move {
        loop {
            select! {
                // Setup outbound channel
                Some(payload) = rx_to_outbound.recv() => {
                    let message: IntraNodePayload = payload;
                    match message.message_type {
                        MessageType::Broadcast => {
                            let gossip_message = bincode2::serialize(&message.payload).expect("Should serialize");
                            if let Err(e) = swarm
                                .behaviour_mut().gossipsub
                                .publish(message.topic, gossip_message) {
                                logger.error(format!("Publish error: {e:?}"));
                            }
                        },

                        MessageType::P2P(peer_id) => {
                            // Send the outer payload in order to attach the topic to it
                            // "Requests are sent using Behaviour::send_request and the responses
                            // received as Message::Response via Event::Message."
                            swarm.behaviour_mut().p2p.send_request(&peer_id, message.payload);
                        }
                    }
                }
                event = swarm.select_next_some() => match event {
                    // Handle P2P messages (all are of type "response" when using the request-response protocol)
                    SwarmEvent::Behaviour(MyBehaviourEvent::P2p(request_response::Event::Message { peer: peer_id, message: request_response::Message::Response {request_id: _, response  } })) => {
                        match response {
                            GossipMessage::Handshake { ecdsa_public_key } => {
                                logger.debug(format!("Got handshake from peer: {peer_id}"));
                                ecdsa_peer_id_to_libp2p_id.write().await.insert(ecdsa_public_key, peer_id);
                            },
                            GossipMessage::Message { topic, raw_payload } => {
                                let topic = IdentTopic::new(topic);
                                if let Some((_, tx, _)) = inbound_mapping.iter().find(|r| r.0.to_string() == topic.to_string()) {
                                    if let Err(e) = tx.send(raw_payload) {
                                        logger.error(format!("Failed to send message to worker: {e}"));
                                    }
                                } else {
                                    logger.error(format!("No registered worker for topic: {topic}!"));
                                }
                            }
                        }
                    }
                    SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed { peer_id, topic })) => {
                        let topic = IdentTopic::new(topic.into_string());
                        logger.debug(format!("Subscribed to topic: {topic} with peer: {peer_id}"));
                        if let Some((_, _, connected_peers)) = inbound_mapping.iter().find(|r| r.0.to_string() == topic.to_string()) {
                            connected_peers.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        } else {
                            logger.error(format!("No registered worker for topic: {topic}!"));
                        }
                    },
                    SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Unsubscribed { peer_id, topic })) => {
                        let topic = IdentTopic::new(topic.into_string());
                        logger.debug(format!("Unsubscribed from topic: {topic} with peer: {peer_id}"));
                        if let Some((_, _, connected_peers)) = inbound_mapping.iter().find(|r| r.0.to_string() == topic.to_string()) {
                            connected_peers.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                        } else {
                            logger.error(format!("No registered worker for topic: {topic}!"));
                        }

                    },
                    SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                        for (peer_id, multiaddr) in list {
                            logger.debug(format!("mDNS discovered a new peer: {peer_id}"));
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                            if let Err(err) = swarm.dial(multiaddr) {
                                logger.error(format!("Failed to dial peer: {err}"));
                            }
                        }
                    },
                    SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                        for (peer_id, _multiaddr) in list {
                            logger.debug(format!("mDNS discover peer has expired: {peer_id}"));
                            swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                        }
                    },

                    // Handle inbound broadcast messages
                    SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    })) => {
                        logger.debug(format!(
                            "Got message: with id: {id} from peer: {peer_id}",
                        ));

                        match bincode2::deserialize::<GossipMessage>(&message.data) {
                            Ok(message) => {
                                match message {
                                    GossipMessage::Handshake { ecdsa_public_key } => {
                                        logger.debug(format!("Got handshake from peer: {peer_id}"));
                                        ecdsa_peer_id_to_libp2p_id.write().await.insert(ecdsa_public_key, peer_id);
                                    },
                                    GossipMessage::Message { topic, raw_payload } => {
                                        if let Some((_, tx, _)) = inbound_mapping.iter().find(|r| r.0.to_string() == topic) {
                                            if let Err(e) = tx.send(raw_payload) {
                                                logger.error(format!("Failed to send message to worker: {e}"));
                                            }
                                        } else {
                                            logger.error(format!("No registered worker for topic: {topic}!"));
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                logger.error(format!("Failed to deserialize message: {e}"));
                            }
                        }
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        logger.debug(format!("Local node is listening on {address}"));
                    }
                    _ => {}
                }
            }
        }
    };

    let spawn_handle = tokio::task::spawn(worker);
    Ok((handles_ret, spawn_handle))
}

#[derive(Clone)]
pub struct GossipHandle {
    topic: IdentTopic,
    tx_to_outbound: UnboundedSender<IntraNodePayload>,
    rx_from_inbound: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>>>,
    logger: DebugLogger,
    connected_peers: Arc<AtomicU32>,
    ecdsa_peer_id_to_libp2p_id:
        Arc<tokio::sync::RwLock<std::collections::HashMap<ecdsa::Public, PeerId>>>,
}

impl GossipHandle {
    pub fn connected_peers(&self) -> usize {
        self.connected_peers
            .load(std::sync::atomic::Ordering::Relaxed) as usize
    }
}

pub struct IntraNodePayload {
    topic: IdentTopic,
    payload: GossipMessage,
    message_type: MessageType,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum GossipMessage {
    Message { topic: String, raw_payload: Vec<u8> },
    Handshake { ecdsa_public_key: ecdsa::Public },
}

enum MessageType {
    Broadcast,
    P2P(PeerId),
}

#[async_trait]
impl Network for GossipHandle {
    async fn next_message(&self) -> Option<<WorkManager as WorkManagerInterface>::ProtocolMessage> {
        let mut lock = self
            .rx_from_inbound
            .try_lock()
            .expect("There should be only a single caller for `next_message`");

        let message = lock.recv().await?;
        match bincode2::deserialize(&message) {
            Ok(message) => Some(message),
            Err(e) => {
                self.logger
                    .error(format!("Failed to deserialize message: {e}"));
                drop(lock);
                self.next_message().await
            }
        }
    }

    async fn send_message(
        &self,
        message: <WorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), gadget_common::Error> {
        let message_type = if let Some(to) = message.to_network_id {
            let libp2p_id = self
                .ecdsa_peer_id_to_libp2p_id
                .read()
                .await
                .get(&to)
                .cloned()
                .ok_or_else(|| gadget_common::Error::NetworkError {
                    err: format!(
                        "No libp2p ID found for ecdsa public key: {to}. No handshake happened?"
                    ),
                })?;

            MessageType::P2P(libp2p_id)
        } else {
            MessageType::Broadcast
        };

        let payload_inner = GossipMessage::Message {
            topic: self.topic.to_string(),
            raw_payload: bincode2::serialize(&message).expect("Should serialize"),
        };

        let payload = IntraNodePayload {
            topic: self.topic.clone(),
            payload: payload_inner,
            message_type,
        };

        self.tx_to_outbound
            .send(payload)
            .map_err(|e| gadget_common::Error::NetworkError {
                err: format!("Failed to send intra-node payload: {e}"),
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{KeystoreConfig, ShellConfig, SubxtConfig};
    use crate::network::setup_network;
    use crate::shell::wait_for_connection_to_bootnodes;
    use gadget_common::prelude::{DebugLogger, GadgetProtocolMessage, Network, WorkManager};
    use gadget_core::job_manager::WorkManagerInterface;

    #[tokio::test]
    async fn test_gossip_network() {
        color_eyre::install().unwrap();
        test_utils::setup_log();
        const N_PEERS: usize = 3;
        let networks = vec!["/test-network"];
        let mut all_handles = Vec::new();
        for x in 0..N_PEERS {
            let identity = libp2p::identity::Keypair::generate_ed25519();

            let logger = DebugLogger {
                peer_id: identity.public().to_peer_id().to_string(),
            };

            let (bind_port, bootnodes) = if x == 0 {
                (
                    30555,
                    vec![
                        "/ip4/0.0.0.0/tcp/30556".parse().unwrap(),
                        "/ip4/0.0.0.0/tcp/30557".parse().unwrap(),
                    ],
                )
            } else if x == 1 {
                (
                    30556,
                    vec![
                        "/ip4/0.0.0.0/tcp/30555".parse().unwrap(),
                        "/ip4/0.0.0.0/tcp/30557".parse().unwrap(),
                    ],
                )
            } else if x == 2 {
                (
                    30557,
                    vec![
                        "/ip4/0.0.0.0/tcp/30555".parse().unwrap(),
                        "/ip4/0.0.0.0/tcp/30556".parse().unwrap(),
                    ],
                )
            } else {
                panic!("Invalid peer index");
            };

            let shell_config = ShellConfig {
                keystore: KeystoreConfig::InMemory,
                subxt: SubxtConfig {
                    endpoint: url::Url::from_directory_path("/").unwrap(),
                },
                bind_ip: "0.0.0.0".to_string(),
                bind_port,
                bootnodes,
                genesis_hash: [0u8; 32],
                node_key: [0u8; 32],
            };

            let (handles, _) =
                setup_network(identity, &shell_config, logger.clone(), networks.clone())
                    .await
                    .unwrap();
            all_handles.push((handles, shell_config, logger));
        }

        for (handles, shell_config, logger) in &all_handles {
            wait_for_connection_to_bootnodes(shell_config, handles, logger)
                .await
                .unwrap();
        }

        // Now, send broadcast messages through each topic
        for _network in networks {
            for (handles, _, _) in &all_handles {
                for handle in handles {
                    handle
                        .send_message(dummy_message(b"Hello, world".to_vec()))
                        .await
                        .unwrap();
                }
            }
        }
    }

    fn dummy_message(input: Vec<u8>) -> <WorkManager as WorkManagerInterface>::ProtocolMessage {
        GadgetProtocolMessage {
            associated_block_id: 0,
            associated_session_id: 0,
            associated_retry_id: 0,
            task_hash: [0u8; 32],
            from: 0,
            to: None,
            payload: input,
            from_network_id: None,
            to_network_id: None,
        }
    }
}
