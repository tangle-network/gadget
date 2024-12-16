#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
mod k256_ecdsa;
use k256::ecdsa::VerifyingKey;
pub use k256_ecdsa::*;

impl From<K256VerifyingKey> for VerifyingKey {
    fn from(key: K256VerifyingKey) -> Self {
        key.0
    }
}
