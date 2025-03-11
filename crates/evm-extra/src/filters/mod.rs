//! Filter creation utilities for EVM events
//!
//! Provides utilities for creating RPC filters for EVM event subscriptions.
//! These filters are used directly with the RPC provider to efficiently
//! filter events at the node level.

use alloy_primitives::Address;
use alloy_rpc_types::Filter;
use alloy_sol_types::SolEvent;

/// Filter creation utilities for EVM contracts
pub mod contract;
/// Filter creation utilities for EVM events
pub mod event;

/// Creates a filter for a specific contract and event type
#[must_use]
pub fn create_event_filter<E: SolEvent>(address: Address) -> Filter {
    Filter::new()
        .address(address)
        .event_signature(E::SIGNATURE_HASH)
}

/// Creates a filter for multiple event types from a single contract
#[must_use]
pub fn create_contract_filter(address: Address, event_signatures: &[&[u8; 32]]) -> Filter {
    Filter::new().address(address).events(event_signatures)
}

/// Creates a filter for a specific event type from any contract
#[must_use]
pub fn create_event_type_filter<E: SolEvent>() -> Filter {
    Filter::new().event_signature(E::SIGNATURE_HASH)
}
