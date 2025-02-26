use crate::error::Error;
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
use gadget_crypto::KeyType;
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

/// Events that can be emitted by the `GadgetBehavior`
#[derive(Debug)]
pub enum GadgetEvent<K: KeyType> {
    /// Discovery-related events
    Discovery(DiscoveryEvent),
    /// Ping events for connection liveness
    Ping(ping::Event),
    /// Blueprint protocol events
    Blueprint(BlueprintProtocolEvent<K>),
}

#[derive(NetworkBehaviour)]
pub struct GadgetBehaviour<K: KeyType> {
    /// Connection limits to prevent `DoS`
    connection_limits: connection_limits::Behaviour,
    /// Discovery mechanisms (Kademlia, mDNS, etc)
    pub(super) discovery: DiscoveryBehaviour<K>,
    /// Direct P2P messaging and gossip
    pub(super) blueprint_protocol: BlueprintProtocolBehaviour<K>,
    /// Connection liveness checks
    ping: ping::Behaviour,
}

impl<K: KeyType> GadgetBehaviour<K> {
    /// Create a new `GadgetBehaviour`
    ///
    /// # Errors
    ///
    /// See [`DiscoveryConfig::build()`]
    pub fn new(
        network_name: &str,
        blueprint_protocol_name: &str,
        local_key: &Keypair,
        instance_key_pair: &K::Secret,
        target_peer_count: u32,
        peer_manager: Arc<PeerManager<K>>,
        protocol_message_sender: Sender<ProtocolMessage<K>>,
        use_address_for_handshake_verification: bool,
    ) -> Result<Self, Error> {
        let connection_limits = connection_limits::Behaviour::new(
            ConnectionLimits::default()
                .with_max_pending_incoming(Some(target_peer_count))
                .with_max_pending_outgoing(Some(target_peer_count))
                .with_max_established_incoming(Some(target_peer_count))
                .with_max_established_outgoing(Some(target_peer_count))
                .with_max_established_per_peer(Some(target_peer_count)),
        );

        let ping = ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(30)));

        info!(
            "Setting up discovery behavior with network name: {}",
            network_name
        );
        let discovery = DiscoveryConfig::new(local_key.public(), network_name)
            .mdns(true)
            .kademlia(true)
            .target_peer_count(target_peer_count)
            .build()?;

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
            use_address_for_handshake_verification,
        );

        debug!("Created GadgetBehaviour with all components initialized");
        Ok(Self {
            connection_limits,
            discovery,
            blueprint_protocol,
            ping,
        })
    }

    /// Bootstrap Kademlia network
    ///
    /// # Errors
    ///
    /// See [`DiscoveryBehaviour::bootstrap()`]
    pub fn bootstrap(&mut self) -> Result<QueryId, Error> {
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
