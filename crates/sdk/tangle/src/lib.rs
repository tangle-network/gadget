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
pub use gadget_event_listeners as event_listeners;

/// Procedural and derive macros for blueprint development
pub use gadget_macros as macros;

/// Core cryptographic primitives and utilities
pub use gadget_crypto as crypto;

/// Tangle-specific cryptographic signing capabilities
pub use gadget_crypto_tangle_pair_signer as crypto_tangle_pair_signer;

/// Configuration management for blueprints
pub use gadget_config as config;

/// Structured logging facilities
pub use gadget_logging as logging;

/// Blueprint execution and runtime utilities
pub use gadget_runners as runners;

/// Local database storage implementations
pub use gadget_store_local_database as store_local_database;
