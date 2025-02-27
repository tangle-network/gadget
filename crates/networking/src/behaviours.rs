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

/// Configuration for `GadgetBehaviour`
pub struct GadgetBehaviourConfig<K: KeyType> {
    /// Name of the network
    pub network_name: String,
    /// Name of the blueprint protocol
    pub blueprint_protocol_name: String,
    /// Local libp2p keypair
    pub local_key: Keypair,
    /// Instance keypair for signing messages
    pub instance_key_pair: K::Secret,
    /// Target number of peers to maintain
    pub target_peer_count: u32,
    /// Peer manager instance
    pub peer_manager: Arc<PeerManager<K>>,
    /// Channel for sending protocol messages
    pub protocol_message_sender: Sender<ProtocolMessage<K>>,
    /// Whether to use EVM address for handshake verification
    pub using_evm_address_for_handshake_verification: bool,
}

impl<K: KeyType> GadgetBehaviour<K> {
    /// Create a new `GadgetBehaviour`
    ///
    /// # Errors
    ///
    /// See [`DiscoveryConfig::build()`]
    pub fn new(config: GadgetBehaviourConfig<K>) -> Result<Self, Error> {
        let connection_limits = connection_limits::Behaviour::new(
            ConnectionLimits::default()
                .with_max_pending_incoming(Some(config.target_peer_count))
                .with_max_pending_outgoing(Some(config.target_peer_count))
                .with_max_established_incoming(Some(config.target_peer_count))
                .with_max_established_outgoing(Some(config.target_peer_count))
                .with_max_established_per_peer(Some(config.target_peer_count)),
        );

        let ping = ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(30)));

        info!(
            "Setting up discovery behavior with network name: {}",
            config.network_name
        );
        let discovery = DiscoveryConfig::new(config.local_key.public(), config.network_name)
            .mdns(true)
            .kademlia(true)
            .target_peer_count(config.target_peer_count)
            .build()?;

        info!(
            "Setting up blueprint protocol with name: {}",
            config.blueprint_protocol_name
        );
        let blueprint_protocol = BlueprintProtocolBehaviour::new(
            &config.local_key,
            &config.instance_key_pair,
            config.peer_manager,
            &config.blueprint_protocol_name,
            config.protocol_message_sender,
            config.using_evm_address_for_handshake_verification,
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
