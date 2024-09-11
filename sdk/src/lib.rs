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
pub mod events_watcher;

/// Gadget Environment Module
pub mod env;

/// Local database storage
pub mod store;

/// Transaction Management Module
pub mod tx;

pub use tangle_subxt;

/// Network Module
#[cfg(feature = "std")]
pub mod network;

pub mod slashing;

/// Benchmark Module
#[cfg(feature = "std")]
pub mod benchmark;

/// Gadget Runner Module
pub mod run;

pub use gadget_blueprint_proc_macro::*;

pub fn setup_log() {
    use tracing_subscriber::fmt::SubscriberBuilder;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    let _ = SubscriberBuilder::default()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init();

    std::panic::set_hook(Box::new(|info| {
        log::error!(target: "gadget", "Panic occurred: {info:?}");
    }));
}
