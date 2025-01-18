//! Blueprint SDK

#[cfg(feature = "tangle")]
pub mod tangle {
    pub use tangle_blueprint_sdk::*;
}

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

pub use gadget_config as config;
pub use gadget_keystore as keystore;
pub use sp_core;
pub use tokio;
