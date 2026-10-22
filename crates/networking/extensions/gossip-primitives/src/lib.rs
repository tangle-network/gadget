//! # Blueprint Gossip Primitives
//!
//! Reusable abstractions for building gossip-based protocols in the Blueprint SDK.
//!
//! This crate provides:
//! - [`ProtocolNetwork`] trait for abstracting network operations
//! - [`GossipManager`] for message deduplication and reliable delivery
//! - [`MessageStore`] for efficient message storage with indexing
//! - Mock implementations for testing without real network dependencies
//!
//! ## Design Principles
//!
//! 1. **Protocol Agnostic**: These primitives work with any message type
//! 2. **Testable**: Mock implementations allow unit testing without mDNS/libp2p
//! 3. **Event-Driven**: Async streams instead of polling for efficient message handling
//! 4. **Memory Bounded**: LRU caches prevent unbounded memory growth
//!
//! ## Example
//!
//! ```rust,ignore
//! use blueprint_gossip_primitives::{ProtocolNetwork, GossipManager, GossipConfig};
//!
//! // Create a gossip manager with deduplication
//! let config = GossipConfig::default();
//! let mut gossip = GossipManager::new(config);
//!
//! // Check if message should be processed (not a duplicate)
//! if gossip.should_process(&message_hash) {
//!     // Process and re-gossip
//!     gossip.mark_seen(message_hash);
//! }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod dedup;
pub mod error;
pub mod network;
pub mod store;

#[cfg(any(test, feature = "testing"))]
pub mod mock;

// Re-exports
pub use dedup::{DeduplicationCache, GossipConfig, GossipManager};
pub use error::GossipError;
pub use network::{MessageStream, NetworkEvent, ProtocolNetwork};
pub use store::{MessageEntry, MessageStore};

#[cfg(any(test, feature = "testing"))]
pub use mock::{MockNetwork, MockNetworkConfig};
