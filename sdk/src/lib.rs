#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    unsafe_code,
    unstable_features,
    unused_results
)]
//! Gadget SDK

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// Keystore Module
pub mod keystore;

/// Metrics Module
// pub mod metrics;

/// Randomness generation module
pub mod random;

/// Blockchain Events Watcher Module
#[cfg(any(feature = "std", feature = "wasm"))]
pub mod events_watcher;

/// Gadget Environment Module
pub mod env;

/// Local database storage
#[cfg(feature = "std")]
pub mod store;

/// Transaction Management Module
#[cfg(any(feature = "std", feature = "wasm"))]
pub mod tx;

pub use tangle_subxt;

/// Network Module
#[cfg(feature = "std")]
pub mod network;

pub mod slashing;

/// Benchmark Module
#[cfg(feature = "std")]
pub mod benchmark;

// TODO: Currently uses `StdGadgetConfiguration`
/// Gadget Runner Module
#[cfg(feature = "std")]
pub mod run;

pub use gadget_blueprint_proc_macro::*;
