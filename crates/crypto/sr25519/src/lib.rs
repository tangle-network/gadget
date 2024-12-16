#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
mod schnorrkel_sr25519;
pub use schnorrkel_sr25519::*;
