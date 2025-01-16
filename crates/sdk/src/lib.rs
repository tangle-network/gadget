//! Blueprint SDK

#[cfg(feature = "tangle")]
pub use tangle_deps::*;
#[cfg(feature = "tangle")]
mod tangle_deps {
    pub use tangle_blueprint_sdk::*;
}

#[cfg(feature = "testing")]
pub use testing::*;
#[cfg(feature = "testing")]
/// Testing utilities and helpers
mod testing {
    /// Tangle-specific client for testing
    pub use gadget_client_tangle;
    /// Tangle-specific testing utilities
    pub use gadget_tangle_testing_utils;
    /// General testing utilities for blueprints
    pub use gadget_testing_utils;
    /// Temporary file and directory management for tests
    pub use tempfile;
}

// Build utilities
#[cfg(feature = "build")]
pub use build::*;
#[cfg(feature = "build")]
/// Build-time utilities for blueprint compilation
mod build {
    /// Build utilities for blueprint compilation
    pub use blueprint_build_utils;
    /// Metadata generation for blueprints
    pub use blueprint_metadata;
}

#[cfg(feature = "networking")]
pub use networking::*;
#[cfg(feature = "networking")]
/// Networking utilities for blueprints
mod networking {
    /// Networking utilities for blueprints
    pub use gadget_networking;
}

pub use gadget_config;
pub use gadget_keystore;
pub use sp_core;
pub use tokio;
