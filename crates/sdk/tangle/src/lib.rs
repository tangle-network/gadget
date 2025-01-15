//! Tangle Blueprint SDK
//! 
//! This SDK provides a comprehensive set of tools and utilities for building Tangle Network blueprints.
//! It includes core functionality for event handling, cryptography, configuration, logging, and optional
//! networking capabilities.
//! 
//! # Features
//! 
//! - `std`: Enables standard library support (default)
//! - `networking`: Enables networking capabilities through `gadget_networking`
//! - `testing`: Enables testing utilities
//! - `build`: Enables build-time utilities
//! - `round-based-compat`: Enables round-based compatibility features for networking

#![cfg_attr(not(feature = "std"), no_std)]

// Core dependencies
/// Event listener infrastructure for handling blueprint events
pub use gadget_event_listeners;

/// Procedural and derive macros for blueprint development
pub use gadget_macros;

/// Core cryptographic primitives and utilities
pub use gadget_crypto;

/// Tangle-specific cryptographic signing capabilities
pub use gadget_crypto_tangle_pair_signer;

/// Configuration management for blueprints
pub use gadget_config;

/// Structured logging facilities
pub use gadget_logging;

/// Blueprint execution and runtime utilities
pub use gadget_runners;

/// Local database storage implementations
pub use gadget_store_local_database;

// Optional networking module
#[cfg(feature = "networking")]
/// Networking capabilities for inter-blueprint communication
pub use gadget_networking;

// Testing utilities
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
    /// Metadata generation for blueprints
    pub use blueprint_metadata;
    /// Build utilities for blueprint compilation
    pub use blueprint_build_utils;
}
