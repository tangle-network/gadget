//! Blueprint SDK

#[cfg(feature = "tangle")]
pub use tangle_subxt;

#[cfg(any(feature = "evm", feature = "eigenlayer"))]
pub use alloy;

#[cfg(feature = "eigenlayer")]
pub use eigensdk;

#[cfg(feature = "testing")]
/// Testing utilities and helpers
pub mod testing {
    /// General testing utilities for blueprints
    pub use gadget_testing_utils as utils;
    /// Temporary file and directory management for tests
    pub use tempfile;
}

// Build utilities
#[cfg(feature = "build")]
/// Build-time utilities for blueprint compilation
pub mod build {
    /// Build utilities for blueprint compilation
    pub use blueprint_build_utils as utils;
    /// Metadata generation for blueprints
    pub use blueprint_metadata;
}

#[cfg(feature = "networking")]
/// Networking utilities for blueprints
pub mod networking {
    /// Networking utilities for blueprints
    pub use gadget_networking::*;
    #[cfg(feature = "round-based-compat")]
    pub use gadget_networking_round_based_extension as round_based_compat;
}

/// Event listener infrastructure for handling blueprint events
pub use gadget_event_listeners as event_listeners;

#[cfg(feature = "macros")]
mod macros_feat {
    pub use gadget_macros as macros;
    pub use gadget_macros::job;
    pub use gadget_macros::main;
}
#[cfg(feature = "macros")]
pub use macros_feat::*;

/// Core cryptographic primitives and utilities
pub use gadget_crypto as crypto;

/// Structured logging facilities
pub use gadget_logging as logging;

/// Blueprint execution and runtime utilities
pub use gadget_runners as runners;

pub use gadget_clients as clients;
pub use gadget_contexts as contexts;

pub use gadget_utils as utils;

pub use gadget_config as config;
pub use gadget_keystore as keystore;
pub use gadget_std as std;
pub use serde;
pub use tokio;

/// Error
pub mod error;
pub use error::Error;

#[cfg(feature = "local-store")]
pub use gadget_stores as stores;
