use std::sync::Arc;
use gadget_config::GadgetConfiguration;
use gadget_network::network::NetworkMultiplexer;
use round_based::PartyIndex;
use std::collections::BTreeMap;
use std::sync::Arc;
use subxt_core::utils::AccountId32;

pub struct P2PClient {
    name: proc_macro2::Ident,
    pub config: GadgetConfiguration,
}

impl P2PClient {
    /// Returns the network protocol identifier
    fn network_protocol(&self) -> String {
        let name = stringify!(self.name).to_string();
        format!("/{}/1.0.0", name.to_lowercase())
    }

    /// Creates a network delivery wrapper for MPC communication
    fn create_network_delivery_wrapper<M>(
        &self,
        mux: Arc<NetworkMultiplexer>,
        party_index: PartyIndex,
        task_hash: [u8; 32],
        parties: BTreeMap<PartyIndex, subxt_core::ext::sp_core::ecdsa::Public>,
    ) -> color_eyre::Result<
        crate::network::round_based_compat::NetworkDeliveryWrapper<M>,
        crate::Error,
    >
    where
        M: Clone
        + Send
        + Unpin
        + 'static
        + serde::Serialize
        + serde::de::DeserializeOwned
        + round_based::ProtocolMessage {

    }

    /// Gets the party index from the participants map
    async fn get_party_index(&self) -> color_eyre::Result<PartyIndex, crate::Error> {

    }

    /// Gets the participants in the MPC protocol
    async fn get_participants(
        &self,
        client: &subxt::OnlineClient<crate::clients::tangle::runtime::TangleConfig>,
    ) -> color_eyre::Result<BTreeMap<PartyIndex, AccountId32>, crate::Error> {

    }

    /// Gets the current blueprint ID
    fn blueprint_id(&self) -> color_eyre::Result<u64> {

    }

    /// Gets the party index and operator mapping
    async fn get_party_index_and_operators(
        &self,
    ) -> color_eyre::Result<(
        usize,
        BTreeMap<AccountId32, crate::subxt_core::ext::sp_core::ecdsa::Public>,
    )> {

    }

    /// Gets the ECDSA keys for all current service operators
    async fn current_service_operators_ecdsa_keys(
        &self,
    ) -> color_eyre::Result<BTreeMap<AccountId32, crate::subxt_core::ext::sp_core::ecdsa::Public>> {

    }

    /// Gets the current call ID for this job
    async fn current_call_id(&self) -> color_eyre::Result<u64> {

    }
}