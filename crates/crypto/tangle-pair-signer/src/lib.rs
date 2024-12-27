#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
pub mod tangle_pair_signer;

pub use tangle_pair_signer::*;
