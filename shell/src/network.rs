use crate::config::ShellConfig;
use async_trait::async_trait;
use futures::stream::StreamExt;
use gadget_common::prelude::{DebugLogger, Network, WorkManager};
use gadget_core::job_manager::WorkManagerInterface;
use libp2p::gossipsub::IdentTopic;
use libp2p::{gossipsub, mdns, swarm::NetworkBehaviour, swarm::SwarmEvent};
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
}

pub async fn setup_network(
    identity: libp2p::identity::Keypair,
    config: &ShellConfig,
    logger: DebugLogger,
    networks: Vec<&str>,
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

            // build a gossipsub network behaviour
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
            Ok(MyBehaviour { gossipsub, mdns })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Subscribe to all networks
    let mut inbound_mapping = Vec::new();
    let (tx_to_outbound, mut rx_to_outbound) =
        tokio::sync::mpsc::unbounded_channel::<IntraNodePayload>();
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
        })
    }

    swarm
        .listen_on(format!("/ip4/{}/udp/{}/quic-v1", config.bind_ip, config.bind_port).parse()?)?;
    // swarm.listen_on(format!("/ip4/{}/tcp/{}", config.bind_ip, config.bind_port).parse()?)?;

    // Dial all bootnodes
    /*
    for bootnode in &config.bootnodes {
        swarm.dial(bootnode.clone())?;
    }*/

    let worker = async move {
        loop {
            select! {
                Some(payload) = rx_to_outbound.recv() => {
                    if let Err(e) = swarm
                        .behaviour_mut().gossipsub
                        .publish(payload.topic, payload.payload) {
                        logger.error(format!("Publish error: {e:?}"));
                    }
                }
                event = swarm.select_next_some() => match event {
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
                    SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    })) => {
                        logger.debug(format!(
                            "Got message: with id: {id} from peer: {peer_id}",
                        ));
                        let topic = IdentTopic::new(message.topic.into_string());

                        if let Some((_, tx, _)) = inbound_mapping.iter().find(|r| r.0.to_string() == topic.to_string()) {
                            if let Err(e) = tx.send(message.data) {
                                logger.error(format!("Failed to send message to worker: {e}"));
                            }
                        } else {
                            logger.error(format!("No registered worker for topic: {}!", &topic));
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
}

impl GossipHandle {
    pub fn connected_peers(&self) -> usize {
        self.connected_peers
            .load(std::sync::atomic::Ordering::Relaxed) as usize
    }
}

pub struct IntraNodePayload {
    topic: IdentTopic,
    payload: Vec<u8>,
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
        let payload = IntraNodePayload {
            topic: self.topic.clone(),
            payload: bincode2::serialize(&message).expect("Should serialize"),
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
    use gadget_common::prelude::DebugLogger;

    #[tokio::test]
    async fn test_gossip_network() {
        color_eyre::install().unwrap();
        test_utils::setup_log();
        const N_PEERS: usize = 3;
        let networks = vec!["test-network"];
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
                        "/ip4/127.0.0.1/tcp/30556".parse().unwrap(),
                        "/ip4/127.0.0.1/tcp/30557".parse().unwrap(),
                    ],
                )
            } else if x == 1 {
                (
                    30556,
                    vec![
                        "/ip4/127.0.0.1/tcp/30555".parse().unwrap(),
                        "/ip4/127.0.0.1/tcp/30557".parse().unwrap(),
                    ],
                )
            } else if x == 2 {
                (
                    30557,
                    vec![
                        "/ip4/127.0.0.1/tcp/30555".parse().unwrap(),
                        "/ip4/127.0.0.1/tcp/30556".parse().unwrap(),
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
                bind_ip: "127.0.0.1".to_string(),
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

        for (handles, shell_config, logger) in all_handles {
            wait_for_connection_to_bootnodes(&shell_config, &handles, &logger)
                .await
                .unwrap();
        }
    }
}
