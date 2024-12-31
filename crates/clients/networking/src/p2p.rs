use crate::error::{Error, Result};
use gadget_config::GadgetConfiguration;
use gadget_networking::gossip::GossipHandle;
use gadget_networking::round_based_compat::NetworkDeliveryWrapper;
use gadget_networking::setup::NetworkConfig;
use gadget_networking::{networking::NetworkMultiplexer, round_based};
use gadget_networking::{GossipMsgKeyPair, GossipMsgPublicKey};
use gadget_std::collections::BTreeMap;
use gadget_std::net::IpAddr;
use gadget_std::sync::Arc;
use round_based::PartyIndex;

pub struct P2PClient {
    pub name: proc_macro2::Ident,
    pub config: GadgetConfiguration,
    pub target_addr: IpAddr,
    pub target_port: u16,
    pub gossip_msg_keypair: GossipMsgKeyPair,
}

impl P2PClient {
    pub fn new(
        name: proc_macro2::Ident,
        config: GadgetConfiguration,
        target_addr: IpAddr,
        target_port: u16,
        gossip_msg_keypair: GossipMsgKeyPair,
    ) -> Self {
        Self {
            name,
            config,
            target_addr,
            target_port,
            gossip_msg_keypair,
        }
    }

    pub fn config(&self) -> &GadgetConfiguration {
        &self.config
    }

    /// Returns the network protocol identifier
    pub fn network_protocol(&self, version: Option<String>) -> String {
        let name = self.name.to_string();
        match version {
            Some(v) => format!("/{}/{}", name.to_lowercase(), v),
            None => format!("/{}/1.0.0", name.to_lowercase()),
        }
    }

    pub fn libp2p_identity(&self, ed25519_seed: Vec<u8>) -> Result<libp2p::identity::Keypair> {
        let mut seed_bytes = ed25519_seed;
        let keypair = libp2p::identity::Keypair::ed25519_from_bytes(&mut seed_bytes)
            .map_err(|err| Error::Configuration(err.to_string()))?;
        Ok(keypair)
    }

    /// Returns a new `NetworkConfig` for the current environment.
    pub fn libp2p_network_config<T: Into<String>>(
        &self,
        network_name: T,
        ed25519_seed: Vec<u8>,
    ) -> Result<NetworkConfig> {
        let network_identity = self.libp2p_identity(ed25519_seed)?;
        let network_config = NetworkConfig::new_service_network(
            network_identity,
            self.gossip_msg_keypair.clone(),
            self.config.bootnodes.clone(),
            self.target_port,
            network_name,
        );

        Ok(network_config)
    }

    /// Starts the P2P network and returns the gossip handle
    pub fn start_p2p_network<T: Into<String>>(
        &self,
        network_name: T,
        ed25519_seed: Vec<u8>,
    ) -> Result<GossipHandle> {
        let network_config = self.libp2p_network_config(network_name, ed25519_seed)?;
        match gadget_networking::setup::start_p2p_network(network_config) {
            Ok(handle) => Ok(handle),
            Err(err) => {
                gadget_logging::error!("Failed to start network: {}", err.to_string());
                Err(Error::Protocol(format!("Failed to start network: {err}")))
            }
        }
    }

    /// Creates a network multiplexer backend
    pub fn create_network_multiplexer<T: Into<String>>(
        &self,
        network_name: T,
        ed25519_seed: Vec<u8>,
    ) -> Result<Arc<NetworkMultiplexer>> {
        let handle = self.start_p2p_network(network_name, ed25519_seed)?;
        Ok(Arc::new(NetworkMultiplexer::new(handle)))
    }

    /// Creates a network delivery wrapper
    pub fn create_network_delivery_wrapper<M>(
        &self,
        mux: Arc<NetworkMultiplexer>,
        party_index: PartyIndex,
        task_hash: [u8; 32],
        parties: BTreeMap<PartyIndex, GossipMsgPublicKey>,
    ) -> NetworkDeliveryWrapper<M>
    where
        M: Clone
            + Send
            + Unpin
            + 'static
            + serde::Serialize
            + serde::de::DeserializeOwned
            + round_based::ProtocolMessage,
    {
        NetworkDeliveryWrapper::new(mux, party_index, task_hash, parties)
    }
}
