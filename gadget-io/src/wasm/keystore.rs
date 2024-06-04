pub use crate::shared::keystore::SubstrateKeystore;

use color_eyre;
use sp_core::{ecdsa, sr25519, Pair};
use std::path::Path;

/// Construct a local keystore shareable container
pub struct KeystoreContainer;

/// Configuration of the client keystore.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum KeystoreConfig {
    /// In-memory keystore.
    InMemory { keystore: Vec<String> },
}

impl KeystoreConfig {
    /// Returns the path for the keystore.
    #[allow(dead_code)]
    pub fn path(&self) -> Option<&Path> {
        None
    }
}

impl SubstrateKeystore for KeystoreConfig {
    fn ecdsa_key(&self) -> color_eyre::Result<ecdsa::Pair> {
        let keys = match self {
            KeystoreConfig::InMemory { keystore: keys } => keys,
        };
        let key = hex::decode(keys[0].clone())?;
        let ecdsa_seed: &[u8] = key.as_slice().try_into()?;
        let ecdsa_key = ecdsa::Pair::from_seed_slice(ecdsa_seed)
            .map_err(|e| color_eyre::eyre::eyre!("{e:?}"))?;

        Ok(ecdsa_key)
    }

    fn sr25519_key(&self) -> color_eyre::Result<sr25519::Pair> {
        let keys = match self {
            KeystoreConfig::InMemory { keystore: keys } => keys,
        };
        let key = hex::decode(keys[1].clone())?;
        let sr25519_key = key.as_slice().try_into()?;
        let sr25519_key = sr25519::Pair::from_seed_slice(sr25519_key)
            .map_err(|e| color_eyre::eyre::eyre!("{e:?}"))?;

        Ok(sr25519_key)
    }
}
