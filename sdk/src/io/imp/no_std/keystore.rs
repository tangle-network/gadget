use crate::error::Result;
pub use crate::shared::keystore::SubstrateKeystore;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use sp_core::{ecdsa, sr25519, Pair};

/// Construct a local keystore shareable container
pub struct KeystoreContainer;

/// Configuration of the client keystore.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum KeystoreConfig {
    /// In-memory keystore.
    InMemory { keystore: Vec<String> },
}

impl SubstrateKeystore for KeystoreConfig {
    fn ecdsa_key(&self) -> Result<ecdsa::Pair> {
        let keys = match self {
            KeystoreConfig::InMemory { keystore: keys } => keys,
        };
        let key = hex::decode(keys[0].clone()).map_err(|e| format!("{e:?}"))?;
        let ecdsa_seed: &[u8] = key.as_slice();
        let ecdsa_key = ecdsa::Pair::from_seed_slice(ecdsa_seed).map_err(|e| format!("{e:?}"))?;

        Ok(ecdsa_key)
    }

    fn sr25519_key(&self) -> Result<sr25519::Pair> {
        let keys = match self {
            KeystoreConfig::InMemory { keystore: keys } => keys,
        };
        let key = hex::decode(keys[1].clone()).map_err(|e| format!("{e:?}"))?;
        let sr25519_key = key.as_slice();
        let sr25519_key =
            sr25519::Pair::from_seed_slice(sr25519_key).map_err(|e| format!("{e:?}"))?;

        Ok(sr25519_key)
    }
}
