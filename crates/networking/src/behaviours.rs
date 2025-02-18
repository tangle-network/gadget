use crate::{
    blueprint_protocol::{BlueprintProtocolBehaviour, BlueprintProtocolEvent},
    discovery::{
        behaviour::{DiscoveryBehaviour, DiscoveryEvent},
        config::DiscoveryConfig,
        PeerInfo, PeerManager,
    },
};
use libp2p::{
    connection_limits::{self, ConnectionLimits},
    identity::Keypair,
    kad::QueryId,
    ping,
    swarm::NetworkBehaviour,
    Multiaddr, PeerId,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

const MAX_ESTABLISHED_PER_PEER: u32 = 4;

/// Events that can be emitted by the GadgetBehavior
#[derive(Debug)]
pub enum GadgetEvent {
    /// Discovery-related events
    Discovery(DiscoveryEvent),
    /// Ping events for connection liveness
    Ping(ping::Event),
    /// Blueprint protocol events
    Blueprint(BlueprintProtocolEvent),
}

#[derive(NetworkBehaviour)]
pub struct GadgetBehaviour {
    /// Connection limits to prevent DoS
    connection_limits: connection_limits::Behaviour,
    /// Discovery mechanisms (Kademlia, mDNS, etc)
    discovery: DiscoveryBehaviour,
    /// Direct P2P messaging
    blueprint_protocol: BlueprintProtocolBehaviour,
    /// Connection liveness checks
    ping: ping::Behaviour,
}

impl GadgetBehaviour {
    pub fn new(
        network_name: &str,
        local_key: &Keypair,
        target_peer_count: u64,
        peer_manager: Arc<PeerManager>,
    ) -> Self {
        let connection_limits = connection_limits::Behaviour::new(
            ConnectionLimits::default()
                .with_max_pending_incoming(Some(
                    target_peer_count as u32 * MAX_ESTABLISHED_PER_PEER,
                ))
                .with_max_pending_outgoing(Some(
                    target_peer_count as u32 * MAX_ESTABLISHED_PER_PEER,
                ))
                .with_max_established_incoming(Some(
                    target_peer_count as u32 * MAX_ESTABLISHED_PER_PEER,
                ))
                .with_max_established_outgoing(Some(
                    target_peer_count as u32 * MAX_ESTABLISHED_PER_PEER,
                ))
                .with_max_established_per_peer(Some(MAX_ESTABLISHED_PER_PEER)),
        );

        let ping = ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(30)));

        let discovery = DiscoveryConfig::new(local_key.public(), network_name)
            .with_mdns(true)
            .with_kademlia(true)
            .with_target_peer_count(target_peer_count)
            .build()
            .unwrap();

        let blueprint_protocol = BlueprintProtocolBehaviour::new(local_key, peer_manager);

        Self {
            connection_limits,
            discovery,
            blueprint_protocol,
            ping,
        }
    }

    /// Bootstrap Kademlia network
    pub fn bootstrap(&mut self) -> Result<QueryId, String> {
        self.discovery.bootstrap()
    }

    /// Returns a set of peer ids
    pub fn peers(&self) -> &HashSet<PeerId> {
        self.discovery.get_peers()
    }

    /// Returns a map of peer ids and their multi-addresses
    pub fn peer_addresses(&self) -> HashMap<PeerId, HashSet<Multiaddr>> {
        self.discovery.get_peer_addresses()
    }

    pub fn peer_info(&self, peer_id: &PeerId) -> Option<&PeerInfo> {
        self.discovery.get_peer_info(peer_id)
    }
}
