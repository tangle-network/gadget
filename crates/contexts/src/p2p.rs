use crate::network::NetworkMultiplexer;
use gadget_std::collections::BTreeMap;
use gadget_std::sync::Arc;
use round_based::PartyIndex;
use subxt_core::utils::AccountId32;

/// `MPCContext` trait provides access to MPC (Multi-Party Computation) functionality from the context.
#[async_trait::async_trait]
pub trait P2PContext {
    fn client(&self) -> gadget_clients::network::round_based_p2p::P2PClient;
}
