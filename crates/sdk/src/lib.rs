//! Blueprint SDK

#[cfg(feature = "tangle")]
pub use tangle_blueprint_sdk as tangle;

#[cfg(feature = "testing")]
/// Testing utilities and helpers
pub mod testing {
    /// Tangle-specific client for testing
    pub use gadget_client_tangle as client_tangle;
    /// Tangle-specific testing utilities
    pub use gadget_tangle_testing_utils as tangle_utils;
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

/// Procedural and derive macros for blueprint development
pub use gadget_macros as macros;

/// Core cryptographic primitives and utilities
pub use gadget_crypto as crypto;

/// Structured logging facilities
pub use gadget_logging as logging;

/// Blueprint execution and runtime utilities
pub use gadget_runners as runners;

pub use gadget_config as config;
pub use gadget_keystore as keystore;
pub use tokio;
