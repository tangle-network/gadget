#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "evm", not(feature = "std")))]
compile_error!("evm feature is currently only supported on std");

#[doc(hidden)]
pub mod ext {
    pub use async_trait;
    pub use futures;
    #[cfg(any(feature = "tangle", feature = "evm"))]
    pub use tokio;

    #[cfg(feature = "std")]
    pub use clap;
    #[cfg(feature = "tangle")]
    pub use gadget_blueprint_serde as blueprint_serde;
    #[cfg(any(feature = "tangle", feature = "evm"))]
    pub use gadget_clients as clients;
    pub use gadget_config as config;
    pub use gadget_contexts as contexts;
    #[cfg(feature = "tangle")]
    pub use gadget_crypto as crypto;
    #[cfg(any(feature = "tangle", feature = "evm"))]
    pub use gadget_event_listeners as event_listeners;
    pub use gadget_keystore as keystore;
    #[cfg(any(feature = "tangle", feature = "evm"))]
    pub use gadget_logging as logging;
    pub use gadget_std as std;
    #[cfg(feature = "std")]
    pub use gadget_testing_utils as testing_utils;

    #[cfg(all(feature = "std", feature = "evm"))]
    pub mod evm {
        pub use alloy_network;
        pub use alloy_provider;
        pub use alloy_transport;
    }

    #[cfg(feature = "tangle")]
    pub mod tangle {
        pub use tangle_subxt;
    }
}

pub use gadget_blueprint_proc_macro::*;
pub use gadget_blueprint_proc_macro_core as core;
pub use gadget_context_derive as contexts;
