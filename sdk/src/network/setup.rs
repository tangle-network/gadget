#![allow(unused_results, missing_docs)]
#[cfg(not(target_family = "wasm"))]
use crate::network::gossip::{
    GossipHandle, IntraNodePayload, MyBehaviour, NetworkServiceWithoutSwarm, MAX_MESSAGE_SIZE,
};
use futures::StreamExt;
use gadget_common::config::DebugLogger;

#[cfg(not(target_family = "wasm"))]
use libp2p::{
    gossipsub, gossipsub::IdentTopic, kad::store::MemoryStore, mdns, request_response,
    swarm::dial_opts::DialOpts, StreamProtocol,
};

use libp2p::Multiaddr;
use sp_core::ecdsa;
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::{Mutex, RwLock};
use tokio::task::{spawn, JoinHandle};

/// The version of the gadget sdk
pub const AGENT_VERSION: &str = "tangle/gadget-sdk/1.0.0";
/// The version of the client
pub const CLIENT_VERSION: &str = "1.0.0";

/// The base network configuration for a blueprint's `libp2p` network.
///
/// This configuration is used to setup the `libp2p` network for a blueprint.
/// Construct using [`NetworkConfig::new`] for advanced users or [`NetworkConfig::new_service_network`] ordinarily.
pub struct NetworkConfig {
    pub identity: libp2p::identity::Keypair,
    pub role_key: ecdsa::Pair,
    pub bootnodes: Vec<Multiaddr>,
    pub bind_ip: IpAddr,
    pub bind_port: u16,
    pub topics: Vec<String>,
    pub logger: DebugLogger,
}

impl std::fmt::Debug for NetworkConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkConfig")
            .field("identity", &self.identity)
            .field("bootnodes", &self.bootnodes)
            .field("bind_ip", &self.bind_ip)
            .field("bind_port", &self.bind_port)
            .field("topics", &self.topics)
            .finish_non_exhaustive()
    }
}

impl NetworkConfig {
    /// For advanced use only. Use `NetworkConfig::new_service_network` for ordinary use.
    /// This function allows for the creation of a network with multiple topics.
    #[must_use]
    pub fn new(
        identity: libp2p::identity::Keypair,
        role_key: ecdsa::Pair,
        bootnodes: Vec<Multiaddr>,
        bind_ip: IpAddr,
        bind_port: u16,
        topics: Vec<String>,
        logger: DebugLogger,
    ) -> Self {
        Self {
            identity,
            role_key,
            bootnodes,
            bind_ip,
            bind_port,
            topics,
            logger,
        }
    }

    /// When constructing a network for a single service, the service name is used as the network name.
    /// Each service within a blueprint must have a unique network name.
    pub fn new_service_network<T: Into<String>>(
        identity: libp2p::identity::Keypair,
        role_key: ecdsa::Pair,
        bootnodes: Vec<Multiaddr>,
        bind_ip: IpAddr,
        bind_port: u16,
        service_name: T,
        logger: DebugLogger,
    ) -> Self {
        Self::new(
            identity,
            role_key,
            bootnodes,
            bind_ip,
            bind_port,
            vec![service_name.into()],
            logger,
        )
    }
}

/// Start a P2P network with the given configuration.
///
/// Each service will only have one network. It is necessary that each service calling this function
/// uses a distinct network name, otherwise, the network will not be able to distinguish between
/// the different services.
///
/// # Arguments
///
/// * `config` - The network configuration.
///
/// # Errors
///
/// Returns an error if the network setup fails.
pub fn start_p2p_network(config: NetworkConfig) -> Result<GossipHandle, Box<dyn Error>> {
    if config.topics.len() != 1 {
        return Err("Only one network topic is allowed when running this function".into());
    }

    let (networks, _) = multiplexed_libp2p_network(config)?;
    let network = networks.into_iter().next().ok_or("No network found")?.1;
    Ok(network)
}

pub type NetworkResult = Result<(HashMap<String, GossipHandle>, JoinHandle<()>), Box<dyn Error>>;

#[allow(clippy::collapsible_else_if, clippy::too_many_lines)]
#[cfg(not(target_family = "wasm"))]
/// Starts the multiplexed libp2p network with the given configuration.
///
/// # Arguments
///
/// * `config` - The network configuration.
///
/// # Errors
///
/// Returns an error if the network setup fails.
///
/// # Panics
///
/// Panics if the network name is invalid.
pub fn multiplexed_libp2p_network(config: NetworkConfig) -> NetworkResult {
    // Setup both QUIC (UDP) and TCP transports the increase the chances of NAT traversal
    let NetworkConfig {
        identity,
        bootnodes,
        bind_ip,
        bind_port,
        topics,
        logger,
        role_key,
    } = config;

    // Ensure all topics are unique
    let topics_unique = topics
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    if topics_unique.len() != topics.len() {
        return Err("All topics must be unique".into());
    }

    let networks = topics;

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(identity)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true), // Allow port reuse for TCP-hole punching
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_quic_config(|mut config| {
            config.handshake_timeout = Duration::from_secs(30);
            config
        })
        .with_dns()?
        .with_relay_client(libp2p::noise::Config::new, libp2p::yamux::Config::default)?
        .with_behaviour(|key, relay_client| {
            // Set a custom gossipsub configuration
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .protocol_id_prefix("/tangle/gadget-binary-sdk/meshsub")
                .max_transmit_size(MAX_MESSAGE_SIZE)
                .validate_messages()
                .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
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
                        StreamProtocol::try_from_owned(n.clone()).expect("Invalid network name"),
                        request_response::ProtocolSupport::Full,
                    )
                })
                .collect::<Vec<_>>();

            let p2p = request_response::Behaviour::new(protocols, p2p_config);

            // Setup the identify protocol for peers to exchange information about each other, a requirement for kadmelia DHT
            let identify = libp2p::identify::Behaviour::new(
                libp2p::identify::Config::new(CLIENT_VERSION.into(), key.public())
                    .with_agent_version(AGENT_VERSION.into()),
            );

            // Setup kadmelia for DHT for peer discovery over a larger network
            let memory_db = MemoryStore::new(key.public().to_peer_id());
            let kadmelia = libp2p::kad::Behaviour::new(key.public().to_peer_id(), memory_db);

            // Setup dcutr for upgrading existing connections to use relay against the bootnodes when necessary
            // This also provided hole-punching capabilities to attempt to seek a direct connection, and fallback to relaying
            // otherwise.
            // dcutr = direct connection upgrade through relay
            let dcutr = libp2p::dcutr::Behaviour::new(key.public().to_peer_id());

            // Setup relay for using the dcutr-upgraded connections to relay messages for other peers when required
            let relay_config = libp2p::relay::Config::default();
            let relay = libp2p::relay::Behaviour::new(key.public().to_peer_id(), relay_config);

            // Setup ping for liveness checks between connections
            let ping = libp2p::ping::Behaviour::new(libp2p::ping::Config::default());

            Ok(MyBehaviour {
                gossipsub,
                mdns,
                p2p,
                identify,
                kadmelia,
                dcutr,
                relay,
                relay_client,
                ping,
            })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    // Subscribe to all networks
    let mut inbound_mapping = Vec::new();
    let (tx_to_outbound, mut rx_to_outbound) =
        tokio::sync::mpsc::unbounded_channel::<IntraNodePayload>();
    let ecdsa_peer_id_to_libp2p_id = Arc::new(RwLock::new(HashMap::new()));
    let mut handles_ret = HashMap::with_capacity(networks.len());
    for network in networks {
        let topic = IdentTopic::new(network.clone());
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        let (inbound_tx, inbound_rx) = tokio::sync::mpsc::unbounded_channel();
        let connected_peers = Arc::new(AtomicU32::new(0));
        inbound_mapping.push((topic.clone(), inbound_tx, connected_peers.clone()));

        handles_ret.insert(
            network,
            GossipHandle {
                connected_peers,
                topic,
                tx_to_outbound: tx_to_outbound.clone(),
                rx_from_inbound: Arc::new(Mutex::new(inbound_rx)),
                logger: logger.clone(),
                ecdsa_peer_id_to_libp2p_id: ecdsa_peer_id_to_libp2p_id.clone(),
            },
        );
    }

    let mut ips_to_bind_to = vec![bind_ip];

    if let IpAddr::V6(v6_addr) = bind_ip {
        if v6_addr.is_loopback() {
            ips_to_bind_to.push(IpAddr::from_str("127.0.0.1").unwrap());
        } else {
            ips_to_bind_to.push(IpAddr::from_str("0.0.0.0").unwrap());
        }
    } else {
        if bind_ip.is_loopback() {
            ips_to_bind_to.push(IpAddr::from_str("::1").unwrap());
        } else {
            ips_to_bind_to.push(IpAddr::from_str("::").unwrap());
        }
    }

    for addr in ips_to_bind_to {
        let ip_label = if addr.is_ipv4() { "ip4" } else { "ip6" };
        swarm.listen_on(format!("/{ip_label}/{addr}/udp/{bind_port}/quic-v1").parse()?)?;
        swarm.listen_on(format!("/{ip_label}/{addr}/tcp/{bind_port}").parse()?)?;
    }

    // Dial all bootnodes
    for bootnode in &bootnodes {
        swarm.dial(
            DialOpts::unknown_peer_id()
                .address(bootnode.clone())
                .build(),
        )?;
    }

    let worker = async move {
        let span = tracing::debug_span!("network_worker");
        let _enter = span.enter();
        let service = NetworkServiceWithoutSwarm {
            logger: &logger,
            inbound_mapping: &inbound_mapping,
            ecdsa_peer_id_to_libp2p_id,
            role_key: &role_key,
            span: tracing::debug_span!(parent: &span, "network_service"),
        };
        loop {
            select! {
                // Setup outbound channel
                Some(msg) = rx_to_outbound.recv() => {
                    service.with_swarm(&mut swarm).handle_intra_node_payload(msg);
                }
                event = swarm.select_next_some() => {
                    service.with_swarm(&mut swarm).handle_swarm_event(event).await;
                }
            }
        }
    };

    let spawn_handle = spawn(worker);
    Ok((handles_ret, spawn_handle))
}
