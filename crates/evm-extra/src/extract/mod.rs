//! Extraction utilities for EVM-specific data
//!
//! This module provides extractors for EVM blocks and events, following the same
//! pattern as Tangle for consistency and reusability.

pub mod block;
pub mod event;

pub use block::{BlockHash, BlockNumber, BlockTimestamp};
pub use event::{BlockEvents, Events};
