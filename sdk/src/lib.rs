#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    unsafe_code,
    unstable_features,
    unused_results,
    clippy::exhaustive_enums
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
pub mod events_watcher;

/// Gadget Environment Module
pub mod env;

/// Transaction Management Module
pub mod tx;

pub use tangle_subxt;

#[cfg(feature = "std")]
pub mod network;

pub mod slashing;

pub mod benchmark;

pub use gadget_blueprint_proc_macro::*;
