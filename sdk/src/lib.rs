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

#[cfg(feature = "std")]
use crate::events_watcher::Error as EventsWatcherError;
use crate::keystore::Error as KeystoreError;
#[cfg(feature = "std")]
use crate::metrics::Error as MetricsError;
use sp_core::ecdsa;
use thiserror::Error;

/// Benchmark Module
#[cfg(feature = "std")]
pub mod benchmark;
/// Context for the gadget
pub mod context;
/// Gadget Environment Module
pub mod env;
/// Blockchain Events Watcher Module
pub mod events_watcher;
/// Keystore Module
pub mod keystore;
/// Debug logger
pub mod logger;
/// Metrics Module
#[cfg(feature = "std")]
pub mod metrics;
/// Network Module
#[cfg(feature = "std")]
pub mod network;
/// Prometheus metrics configuration
#[cfg(feature = "std")]
pub mod prometheus;
/// Randomness generation module
pub mod random;
/// Gadget Runner Module
pub mod run;
/// Slashing and quality of service utilities
pub mod slashing;
/// Database storage
pub mod store;
/// Protocol execution tracer
pub mod tracer;
/// Transaction Management Module
pub mod tx;

/// Re-exports
pub use gadget_blueprint_proc_macro::*;
pub use tangle_subxt;

/// Represents errors that can occur in the SDK.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Client error: {0}")]
    ClientError(String),

    #[error("Job error: {reason}")]
    JobError { reason: String },

    #[error("Network error: {reason}")]
    NetworkError { reason: String },

    #[error("Keystore error: {0}")]
    KeystoreError(#[from] KeystoreError),

    #[error("Missing network ID")]
    MissingNetworkId,

    #[error("Peer not found: {0:?}")]
    PeerNotFound(ecdsa::Public),

    #[cfg(feature = "std")]
    #[error("Join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),

    #[error("Subxt error: {0}")]
    Subxt(#[from] subxt::Error),

    #[cfg(feature = "std")]
    #[error("Events watcher error: {0}")]
    EventsWatcherError(#[from] EventsWatcherError),

    #[cfg(feature = "std")]
    #[error("Prometheus error: {err}")]
    PrometheusError { err: String },

    #[cfg(feature = "std")]
    #[error("Metrics error: {0}")]
    MetricsError(#[from] MetricsError),

    #[error("Other error: {0}")]
    Other(#[from] String),
}
