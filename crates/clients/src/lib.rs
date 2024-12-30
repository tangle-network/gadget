#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
pub use error::*;

#[cfg(feature = "eigenlayer")]
pub use gadget_client_eigenlayer as eigenlayer;

#[cfg(feature = "evm")]
pub use gadget_client_evm as evm;

#[cfg(feature = "networking")]
pub use gadget_client_networking as networking;

#[cfg(feature = "tangle")]
pub use gadget_client_tangle as tangle;

pub use gadget_client_core::*;
