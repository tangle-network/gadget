#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    rustdoc::broken_intra_doc_links,
    unused_results,
    clippy::all,
    clippy::pedantic,
    clippy::exhaustive_enums
)]
//! Gadget SDK

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

/// Keystore Module
#[cfg(not(feature = "wasm"))]
pub mod keystore;

/// Metrics Module
// #[cfg(feature = "metrics")]
// pub mod metrics;

/// Logging Module
#[cfg(feature = "logging")]
pub mod logging;
/// Randomness generation module
#[cfg(not(feature = "wasm"))]
pub mod random;

/// Blockchain Events Watcher Module
#[cfg(feature = "events-watcher")]
pub mod events_watcher;

/// Gadget Environment Module
pub mod env;

/// Transaction Management Module
pub mod tx;

// Exporting the macros
#[cfg(feature = "macros")]
pub use gadget_blueprint_proc_macro::{job, registration_hook, report, request_hook};

#[doc(hidden)]
#[cfg(feature = "macros")]
pub use tangle_subxt;
#[cfg(feature = "networking-libp2p")]
pub mod network;
