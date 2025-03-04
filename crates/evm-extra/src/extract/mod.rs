//! Extraction utilities for EVM-specific data
//!
//! This module provides extractors for EVM blocks and events, following the same
//! pattern as Tangle for consistency and reusability.

pub mod block;
pub mod contract;
pub mod event;
pub mod tx;

pub use block::{BlockHash, BlockNumber, BlockTimestamp};
pub use contract::ContractAddress;
pub use event::{BlockEvents, Events, FirstEvent, LastEvent};
pub use tx::Tx;
