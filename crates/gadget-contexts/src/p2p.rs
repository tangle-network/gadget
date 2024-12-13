use crate::network::NetworkMultiplexer;
use round_based::PartyIndex;
use std::collections::BTreeMap;
use std::sync::Arc;
use subxt_core::utils::AccountId32;

/// `MPCContext` trait provides access to MPC (Multi-Party Computation) functionality from the context.
#[async_trait::async_trait]
pub trait P2PContext {
    fn client(&self) -> gadget_clients::network::p2p::P2PClient;
}
