#![deny(
    missing_copy_implementations,
    unsafe_code,
    unstable_features,
    unused_results
)]
//! Gadget SDK

#![cfg_attr(all(not(feature = "std"), not(feature = "wasm")), no_std)]

extern crate alloc;
extern crate core;

/// Benchmark Module
#[cfg(any(feature = "std", feature = "wasm"))]
pub mod benchmark;
/// Sol Bindings
#[allow(missing_copy_implementations)]
pub mod binding;
/// Blockchain clients
#[cfg(any(feature = "std", feature = "wasm"))]
pub mod clients;
/// Gadget configuration
pub mod config;
pub mod error;
/// Event Listener Module
#[cfg(any(feature = "std", feature = "wasm"))]
pub mod event_listener;
/// Blockchain Events Watcher Module
#[cfg(any(feature = "std", feature = "wasm"))]
pub mod event_utils;
/// Command execution module
#[cfg(feature = "std")]
pub mod executor;
/// Keystore Module
pub mod keystore;
pub mod logging;
/// Metrics Module
#[cfg(feature = "std")]
pub mod metrics;
#[cfg(any(feature = "std", feature = "wasm"))]
pub mod mutex_ext;
/// Network Module
#[cfg(feature = "std")] // TODO: Eventually open this up to WASM
pub mod network;
/// Prometheus metrics configuration
#[cfg(any(feature = "std", feature = "wasm"))]
pub mod prometheus;
/// Randomness generation module
pub mod random;
/// Gadget Runner Module
#[cfg(feature = "std")] // TODO: Eventually open this up to WASM
pub mod run;
/// Blueprint runners
pub mod runners;
/// Slashing and quality of service utilities
pub mod slashing;
/// Database storage
#[cfg(feature = "std")]
pub mod store;
/// Protocol execution tracer
#[cfg(any(feature = "std", feature = "wasm"))]
pub mod tracer;
/// Transaction Management Module
#[cfg(any(feature = "std", feature = "wasm"))]
pub mod tx;

/// Gadget Context and context extensions
pub mod ctx;

pub mod docker;
pub mod utils;

// Re-exports
pub use alloy_rpc_types;
pub use async_trait;
pub use clap;
pub use error::Error;
pub use futures;
pub use gadget_blueprint_proc_macro::*;
pub use libp2p;
pub use parking_lot;
pub use subxt_core;
pub use tangle_subxt;
pub use tokio;
pub use tracing;
pub use uuid;

// External modules usually used in proc-macro codegen.
#[doc(hidden)]
pub mod ext {
    pub use blueprint_serde;
    pub use lock_api;
    #[cfg(feature = "std")]
    pub use parking_lot;
    pub use sp_core;
    pub use tangle_subxt;
    pub use tangle_subxt::subxt;
    pub use tangle_subxt::subxt_signer;
}
