//! Blueprint SDK
//!
//! ## Features
#![doc = document_features::document_features!()]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://cdn.prod.website-files.com/6494562b44a28080aafcbad4/65aaf8b0818b1d504cbdf81b_Tnt%20Logo.png"
)]

// == Core utilities ==

// Expose the core module to the outside world
pub use blueprint_core::*;

/// Re-export the core extractors from the `blueprint_core` crate.
pub mod extract {
    #[cfg(feature = "macros")]
    pub use blueprint_macros::FromRef;

    pub use blueprint_core::extract::*;
}

#[cfg(feature = "tangle")]
pub use blueprint_tangle_extra as tangle;

#[cfg(feature = "evm")]
pub use blueprint_evm_extra as evm;

pub use blueprint_runner as runner;

pub mod producers {
    #[cfg(feature = "cronjob")]
    pub use blueprint_producers_extra::cron::CronJob;
}

pub use blueprint_router::Router;

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
pub use blueprint_macros::debug_job;

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
