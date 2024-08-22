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
#![allow(clippy::single_match_else)]
//! Gadget SDK

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

/// Keystore Module
#[cfg(not(feature = "wasm"))]
pub mod keystore;

/// Metrics Module
// pub mod metrics;

/// Randomness generation module
#[cfg(not(feature = "wasm"))]
pub mod random;

/// Blockchain Events Watcher Module
pub mod events_watcher;

/// Gadget Environment Module
pub mod env;

/// Transaction Management Module
pub mod tx;

pub use tangle_subxt;

pub mod network;

pub mod slashing;

pub mod benchmark;

pub use gadget_blueprint_proc_macro::*;
