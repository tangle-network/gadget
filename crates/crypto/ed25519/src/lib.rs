#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
mod zebra_ed25519;

pub use zebra_ed25519::*;
