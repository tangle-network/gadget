use super::*;
use gadget_config::GadgetConfiguration;
use gadget_network::network::NetworkMultiplexer;
use round_based::PartyIndex;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Arc;
use subxt_core::utils::AccountId32;
use tangle_subxt::subxt_core;

pub struct P2PClient {
    name: proc_macro2::Ident,
    pub config: GadgetConfiguration,
}

/// Trait for P2P network communication
#[async_trait::async_trait]
pub trait P2PProtocol {
    /// The public key type used for identifying parties
    type PublicKey: Clone + Send + Sync + 'static;

    /// Returns the network protocol identifier
    fn network_protocol(&self) -> String;

    /// Creates a network delivery wrapper for MPC communication
    fn create_network_delivery_wrapper<M>(
        &self,
        mux: Arc<NetworkMultiplexer>,
        party_index: PartyIndex,
        task_hash: [u8; 32],
        parties: BTreeMap<PartyIndex, Self::PublicKey>,
    ) -> Result<crate::network::round_based_compat::NetworkDeliveryWrapper<M>, Error>
    where
        M: Clone
            + Send
            + Unpin
            + 'static
            + serde::Serialize
            + serde::de::DeserializeOwned
            + round_based::ProtocolMessage;

    // TODO: THESE METHODS DONT BELONG, PUT THEM IN RESPECTIVE RESTAKING PROTOCOL CLIENT
    // /// Gets the party index from the participants map
    // async fn get_party_index(&self) -> Result<PartyIndex, Error>;

    // /// Gets the participants in the MPC protocol
    // async fn get_participants(
    //     &self,
    //     client: &subxt::OnlineClient<crate::clients::tangle::runtime::TangleConfig>,
    // ) -> Result<BTreeMap<PartyIndex, AccountId32>, Error>;

    // /// Gets the current blueprint ID
    // fn blueprint_id(&self) -> Result<u64, Error>;

    // /// Gets the party index and operator mapping
    // async fn get_party_index_and_operators(
    //     &self,
    // ) -> Result<(usize, BTreeMap<AccountId32, Self::PublicKey>), Error>;

    // /// Gets the keys for all current service operators
    // async fn current_service_operators_keys(
    //     &self,
    // ) -> Result<BTreeMap<AccountId32, Self::PublicKey>, Error>;

    // /// Gets the current call ID for this job
    // async fn current_call_id(&self) -> Result<u64, Error>;
}
