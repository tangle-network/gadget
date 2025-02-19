use crate::error::Result as NetworkingResult;
use crate::key_types::InstanceMsgKeyPair;
use crate::types::ProtocolMessage;
use crate::{
    blueprint_protocol::{BlueprintProtocolBehaviour, BlueprintProtocolEvent},
    discovery::{
        behaviour::{DiscoveryBehaviour, DiscoveryEvent},
        config::DiscoveryConfig,
        PeerInfo, PeerManager,
    },
};
use crossbeam_channel::Sender;
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
use tracing::{debug, info};

const MAX_ESTABLISHED_PER_PEER: u32 = 4;

/// Events that can be emitted by the `GadgetBehavior`
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
    /// Connection limits to prevent `DoS`
    connection_limits: connection_limits::Behaviour,
    /// Discovery mechanisms (Kademlia, mDNS, etc)
    pub(super) discovery: DiscoveryBehaviour,
    /// Direct P2P messaging and gossip
    pub(super) blueprint_protocol: BlueprintProtocolBehaviour,
    /// Connection liveness checks
    ping: ping::Behaviour,
}

impl GadgetBehaviour {
    #[must_use]
    pub fn new(
        network_name: &str,
        blueprint_protocol_name: &str,
        local_key: &Keypair,
        instance_key_pair: &InstanceMsgKeyPair,
        target_peer_count: u64,
        peer_manager: Arc<PeerManager>,
        protocol_message_sender: Sender<ProtocolMessage>,
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

        info!(
            "Setting up discovery behavior with network name: {}",
            network_name
        );
        let discovery = DiscoveryConfig::new(local_key.public(), network_name)
            .with_mdns(true)
            .with_kademlia(true)
            .with_target_peer_count(target_peer_count)
            .build()
            .unwrap();

        info!(
            "Setting up blueprint protocol with name: {}",
            blueprint_protocol_name
        );
        let blueprint_protocol = BlueprintProtocolBehaviour::new(
            local_key,
            instance_key_pair,
            peer_manager,
            blueprint_protocol_name,
            protocol_message_sender,
        );

        debug!("Created GadgetBehaviour with all components initialized");
        Self {
            connection_limits,
            discovery,
            blueprint_protocol,
            ping,
        }
    }

    /// Bootstrap Kademlia network
    pub fn bootstrap(&mut self) -> NetworkingResult<QueryId> {
        self.discovery.bootstrap()
    }

    /// Returns a set of peer ids
    #[must_use]
    pub fn peers(&self) -> &HashSet<PeerId> {
        self.discovery.get_peers()
    }

    /// Returns a map of peer ids and their multi-addresses
    #[must_use]
    pub fn peer_addresses(&self) -> HashMap<PeerId, HashSet<Multiaddr>> {
        self.discovery.get_peer_addresses()
    }

    #[must_use]
    pub fn peer_info(&self, peer_id: &PeerId) -> Option<&PeerInfo> {
        self.discovery.get_peer_info(peer_id)
    }
}
