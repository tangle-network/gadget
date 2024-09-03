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
use gadget_io::time::error::Elapsed;
use gadget_io::tokio::sync::MutexGuard;
use sp_core::ecdsa;
use std::time::Duration;
use thiserror::Error;

/// Benchmark Module
#[cfg(feature = "std")]
pub mod benchmark;
/// Blockchain clients
pub mod clients;
/// Gadget configuration
pub mod config;
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

    #[error("Storage error: {reason}")]
    StoreError { reason: String },

    #[error("Keystore error: {0}")]
    KeystoreError(#[from] KeystoreError),

    #[error("Missing network ID")]
    MissingNetworkId,

    #[error("Peer not found: {id:?}")]
    PeerNotFound { id: ecdsa::Public },

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
    Other(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}

#[async_trait::async_trait]
pub trait TokioMutexExt<T: Send> {
    async fn try_lock_timeout(&self, timeout: Duration) -> Result<MutexGuard<T>, Elapsed>;
    async fn lock_timeout(&self, timeout: Duration) -> MutexGuard<T> {
        self.try_lock_timeout(timeout)
            .await
            .expect("Timeout on mutex lock")
    }
}

#[async_trait::async_trait]
impl<T: Send> TokioMutexExt<T> for gadget_io::tokio::sync::Mutex<T> {
    async fn try_lock_timeout(&self, timeout: Duration) -> Result<MutexGuard<T>, Elapsed> {
        gadget_io::time::timeout(timeout, self.lock()).await
    }
}
