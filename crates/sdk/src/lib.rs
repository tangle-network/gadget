//! Blueprint SDK

#[cfg(feature = "testing")]
/// Testing utilities and helpers
pub mod testing {
    /// Tangle-specific testing utilities
    #[cfg(feature = "tangle")]
    pub mod tangle {
        pub use gadget_client_tangle as client;
        pub use gadget_tangle_testing_utils::*;
    }

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
}

/// Event listener infrastructure for handling blueprint events
pub use gadget_event_listeners as event_listeners;

#[cfg(feature = "macros")]
pub use gadget_macros as macros;
#[cfg(feature = "macros")]
pub use gadget_macros::main;

/// Core cryptographic primitives and utilities
pub use gadget_crypto as crypto;

/// Structured logging facilities
pub use gadget_logging as logging;

/// Blueprint execution and runtime utilities
pub use gadget_runners as runners;

pub use gadget_config as config;
pub use gadget_keystore as keystore;
pub use tokio;
