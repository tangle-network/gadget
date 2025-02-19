use super::{
    behaviour::{DerivedDiscoveryBehaviour, DiscoveryBehaviour},
    new_kademlia,
};
use libp2p::{
    autonat, identify, identity::PublicKey, mdns, relay, upnp, Multiaddr, PeerId, StreamProtocol,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Duration,
};
use tracing::warn;

pub struct DiscoveryConfig {
    /// The local peer ID.
    local_peer_id: PeerId,
    /// The local public key.
    local_public_key: PublicKey,
    /// The bootstrap peers.
    bootstrap_peers: Vec<(PeerId, Multiaddr)>,
    /// The relay nodes.
    relay_nodes: Vec<(PeerId, Multiaddr)>,
    /// The number of peers to connect to.
    target_peer_count: u64,
    /// Enable mDNS discovery.
    enable_mdns: bool,
    /// Enable Kademlia discovery.
    enable_kademlia: bool,
    /// Enable `UPnP` discovery.
    enable_upnp: bool,
    /// Enable relay nodes.
    enable_relay: bool,
    /// The name of the network.
    network_name: String,
    /// Protocol version string that uniquely identifies your P2P service.
    /// This should be unique to your application to avoid conflicts with other P2P networks.
    /// Format recommendation: "<service-name>/<version>"
    /// Example: "my-blockchain/1.0.0" or "my-chat-app/0.1.0"
    protocol_version: String,
}

impl DiscoveryConfig {
    pub fn new(local_public_key: PublicKey, network_name: impl Into<String>) -> Self {
        Self {
            local_peer_id: local_public_key.to_peer_id(),
            local_public_key,
            bootstrap_peers: Vec::new(),
            relay_nodes: Vec::new(),
            target_peer_count: 25, // Reasonable default
            enable_mdns: true,     // Enable by default for local development
            enable_kademlia: true, // Enable by default for production
            enable_upnp: true,     // Enable by default for better connectivity
            enable_relay: true,    // Enable by default for relay functionality
            network_name: network_name.into(),
            protocol_version: String::from("gadget/1.0.0"), // Default version
        }
    }

    /// Set the protocol version that uniquely identifies your P2P service.
    /// This should be unique to your application to avoid conflicts with other P2P networks.
    /// Format recommendation: "<service-name>/<version>"
    pub fn with_protocol_version(mut self, version: impl Into<String>) -> Self {
        self.protocol_version = version.into();
        self
    }

    #[must_use]
    pub fn with_bootstrap_peers(mut self, peers: Vec<(PeerId, Multiaddr)>) -> Self {
        self.bootstrap_peers = peers;
        self
    }

    #[must_use]
    pub fn with_relay_nodes(mut self, nodes: Vec<(PeerId, Multiaddr)>) -> Self {
        self.relay_nodes = nodes;
        self
    }

    #[must_use]
    pub fn with_target_peer_count(mut self, count: u64) -> Self {
        self.target_peer_count = count;
        self
    }

    #[must_use]
    pub fn with_mdns(mut self, enable: bool) -> Self {
        self.enable_mdns = enable;
        self
    }

    #[must_use]
    pub fn with_kademlia(mut self, enable: bool) -> Self {
        self.enable_kademlia = enable;
        self
    }

    #[must_use]
    pub fn with_upnp(mut self, enable: bool) -> Self {
        self.enable_upnp = enable;
        self
    }

    #[must_use]
    pub fn with_relay(mut self, enable: bool) -> Self {
        self.enable_relay = enable;
        self
    }

    pub fn build(self) -> anyhow::Result<DiscoveryBehaviour> {
        let kademlia_opt = if self.enable_kademlia {
            let protocol = StreamProtocol::try_from_owned(format!(
                "/gadget/kad/{}/kad/1.0.0",
                self.network_name
            ))?;

            let mut kademlia = new_kademlia(self.local_peer_id, protocol);

            // Add bootstrap peers
            for (peer_id, addr) in &self.bootstrap_peers {
                kademlia.add_address(peer_id, addr.clone());
            }

            // Start bootstrap process
            if let Err(e) = kademlia.bootstrap() {
                warn!("Kademlia bootstrap failed: {}", e);
            }

            Some(kademlia)
        } else {
            None
        };

        let mdns_opt = if self.enable_mdns {
            Some(mdns::Behaviour::new(
                Default::default(),
                self.local_peer_id,
            )?)
        } else {
            None
        };

        let upnp_opt = if self.enable_upnp {
            Some(upnp::tokio::Behaviour::default())
        } else {
            None
        };

        let relay_opt = if self.enable_relay {
            let relay = relay::Behaviour::new(self.local_peer_id, Default::default());
            Some(relay)
        } else {
            None
        };

        let behaviour = DerivedDiscoveryBehaviour {
            kademlia: kademlia_opt.into(),
            mdns: mdns_opt.into(),
            identify: identify::Behaviour::new(
                identify::Config::new(self.protocol_version, self.local_public_key)
                    .with_agent_version(format!("gadget-{}", env!("CARGO_PKG_VERSION")))
                    .with_push_listen_addr_updates(true),
            ),
            autonat: autonat::Behaviour::new(self.local_peer_id, Default::default()),
            upnp: upnp_opt.into(),
            relay: relay_opt.into(),
        };

        Ok(DiscoveryBehaviour {
            discovery: behaviour,
            peers: HashSet::new(),
            peer_info: HashMap::new(),
            target_peer_count: self.target_peer_count,
            next_kad_random_query: tokio::time::interval(Duration::from_secs(1)),
            duration_to_next_kad: Duration::from_secs(1),
            pending_events: VecDeque::new(),
            n_node_connected: 0,
            pending_dial_opts: VecDeque::new(),
        })
    }
}
