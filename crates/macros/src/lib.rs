#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "evm", not(feature = "std")))]
compile_error!("evm feature is currently only supported on std");

#[doc(hidden)]
pub mod ext {
    pub use async_trait;

    pub use gadget_config as config;
    pub use gadget_contexts as contexts;
    pub use gadget_keystore as keystore;
    pub use gadget_networking as networking;

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
