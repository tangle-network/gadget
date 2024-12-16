use crate::error::Result;
use gadget_config::GadgetConfiguration;
use gadget_crypto::k256_crypto::K256VerifyingKey;
use gadget_networking::round_based_compat::NetworkDeliveryWrapper;
use gadget_networking::{networking::NetworkMultiplexer, round_based};
use gadget_std::collections::BTreeMap;
use gadget_std::sync::Arc;
use round_based::PartyIndex;

pub struct P2PClient {
    name: proc_macro2::Ident,
    pub config: GadgetConfiguration,
}

impl P2PClient {
    /// Returns the network protocol identifier
    fn network_protocol(&self, version: Option<String>) -> String {
        let name = stringify!(self.name).to_string();
        match version {
            Some(v) => format!("/{}/{}", name.to_lowercase(), v),
            None => format!("/{}/1.0.0", name.to_lowercase()),
        }
    }

    /// Creates a network delivery wrapper for MPC communication
    fn create_network_delivery_wrapper<M>(
        &self,
        mux: Arc<NetworkMultiplexer>,
        party_index: PartyIndex,
        task_hash: [u8; 32],
        parties: BTreeMap<PartyIndex, K256VerifyingKey>,
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
