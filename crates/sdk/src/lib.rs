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

/// Core cryptographic primitives and utilities
pub use gadget_crypto as crypto;

/// Structured logging facilities
pub use gadget_logging as logging;

pub use gadget_clients as clients;
pub use gadget_contexts as contexts;

pub use gadget_utils as utils;

pub use async_trait;
pub use gadget_keystore as keystore;
pub use gadget_std as std;
pub use serde;
pub use tokio;

pub mod error;
pub use error::Error;

/// Re-export the core extractors from the `blueprint_core` crate.
pub mod extract {
    #[cfg(feature = "macros")]
    pub use blueprint_macros::FromRef;

    pub use blueprint_core::extract::*;
}

/// Blueprint execution and runtime utilities
pub use blueprint_runner as runner;

pub mod producers {
    #[cfg(feature = "cronjob")]
    pub use blueprint_producers_extra::cron::CronJob;
}

pub use blueprint_router::Router;

#[cfg(feature = "macros")]
pub mod macros {
    pub use blueprint_macros::*;
    pub use gadget_context_derive as context;
}

// == Protocol-specific utilities ==

#[cfg(feature = "tangle")]
mod tangle_feat {
    pub use blueprint_tangle_extra as tangle;
    pub use tangle_subxt;
}
#[cfg(feature = "tangle")]
pub use tangle_feat::*;

#[cfg(any(feature = "evm", feature = "eigenlayer"))]
mod evm_feat {
    pub use alloy;
    pub use blueprint_evm_extra as evm;
}
#[cfg(any(feature = "evm", feature = "eigenlayer"))]
pub use evm_feat::*;

#[cfg(feature = "eigenlayer")]
pub use eigensdk;

// == Development utilities ==

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
}

#[cfg(feature = "networking")]
/// Networking utilities for blueprints
pub mod networking {
    /// Networking utilities for blueprints
    pub use gadget_networking::*;
    #[cfg(feature = "round-based-compat")]
    pub use gadget_networking_round_based_extension as round_based_compat;
}

#[cfg(feature = "local-store")]
pub use gadget_stores as stores;
