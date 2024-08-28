use crate::io::error::{GadgetIoError, Result};
use crate::io::SubstrateKeystore;
use sp_core::{ecdsa, sr25519, ByteArray};
use std::path::{Path, PathBuf};
use tracing;

use sc_keystore::{Keystore, LocalKeystore};
use sp_keystore::KeystorePtr;
use std::sync::Arc;

/// Construct a local keystore shareable container
#[allow(missing_debug_implementations)]
pub struct KeystoreContainer(Arc<LocalKeystore>);

impl KeystoreContainer {
    /// Construct KeystoreContainer
    pub fn new(config: &KeystoreConfig) -> Result<Self> {
        let keystore = Arc::new(match config {
            KeystoreConfig::Path { path, password } => {
                LocalKeystore::open(path.clone(), password.clone())
                    .map_err(|e| GadgetIoError::Other(e.to_string()))?
            }
            KeystoreConfig::InMemory => LocalKeystore::in_memory(),
        });

        Ok(Self(keystore))
    }

    /// Returns a shared reference to a dynamic `Keystore` trait implementation.
    #[allow(dead_code)]
    pub fn keystore(&self) -> KeystorePtr {
        self.0.clone()
    }

    /// Returns a shared reference to the local keystore .
    pub fn local_keystore(&self) -> Arc<LocalKeystore> {
        self.0.clone()
    }
}

/// Configuration of the client keystore.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum KeystoreConfig {
    /// Keystore at a path on-disk. Recommended for native gadgets.
    Path {
        /// The path of the keystore.
        path: PathBuf,
        /// keystore's password.
        password: Option<sp_core::crypto::SecretString>,
    },
    /// In-memory keystore.
    InMemory,
}

impl KeystoreConfig {
    /// Returns the path for the keystore.
    #[allow(dead_code)]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Path { path, .. } => Some(path),
            Self::InMemory => None,
        }
    }
}

pub mod crypto {
    use sp_application_crypto::{app_crypto, ecdsa, sr25519};
    pub mod acco {
        use super::*;
        pub use sp_core::crypto::key_types::ACCOUNT as KEY_TYPE;
        app_crypto!(sr25519, KEY_TYPE);
    }

    pub mod role {
        use super::*;
        /// Key type for ROLE keys
        pub const KEY_TYPE: sp_application_crypto::KeyTypeId =
            sp_application_crypto::KeyTypeId(*b"role");

        app_crypto!(ecdsa, KEY_TYPE);
    }
}

impl SubstrateKeystore for KeystoreConfig {
    // TODO: Return more specific error types
    fn ecdsa_key(&self) -> Result<ecdsa::Pair> {
        let keystore_container = KeystoreContainer::new(self)?;
        let keystore = keystore_container.local_keystore();
        tracing::debug!("Loaded keystore from path");
        let ecdsa_keys = keystore.ecdsa_public_keys(crypto::role::KEY_TYPE);

        if ecdsa_keys.len() != 1 {
            return Err(GadgetIoError::Other(format!(
                "`role`: Expected exactly one key in ECDSA keystore, found {}",
                ecdsa_keys.len()
            )));
        }

        let role_public_key = crypto::role::Public::from_slice(&ecdsa_keys[0].0).map_err(|_| {
            GadgetIoError::Other(String::from("Failed to parse public key from keystore"))
        })?;

        let role_key = keystore
            .key_pair::<crypto::role::Pair>(&role_public_key)
            .map_err(|e| GadgetIoError::Other(e.to_string()))?
            .ok_or(GadgetIoError::Other(String::from(
                "Failed to load key `role` from keystore",
            )))?
            .into_inner();

        tracing::debug!(%role_public_key, "Loaded key from keystore");
        println!("ECDSA KEY: {:?}", role_public_key.to_string());
        println!("ECDSA SEED: {:?}", role_key.seed());
        println!("ECDSA RAW: {:?}", ecdsa_keys);
        Ok(role_key)
    }

    // TODO: Return more specific error types
    fn sr25519_key(&self) -> Result<sr25519::Pair> {
        let keystore_container = KeystoreContainer::new(self)?;
        let keystore = keystore_container.local_keystore();
        tracing::debug!("Loaded keystore from path");
        let sr25519_keys = keystore.sr25519_public_keys(crypto::acco::KEY_TYPE);

        if sr25519_keys.len() != 1 {
            return Err(GadgetIoError::Other(format!(
                "`acco`: Expected exactly one key in SR25519 keystore, found {}",
                sr25519_keys.len()
            )));
        }

        let account_public_key =
            crypto::acco::Public::from_slice(&sr25519_keys[0].0).map_err(|_| {
                GadgetIoError::Other(String::from("Failed to parse public key from keystore"))
            })?;

        let acco_key = keystore
            .key_pair::<crypto::acco::Pair>(&account_public_key)
            .map_err(|e| GadgetIoError::Other(e.to_string()))?
            .ok_or(GadgetIoError::Other(String::from(
                "Failed to load key `acco` from keystore",
            )))?
            .into_inner();

        tracing::debug!(%account_public_key, "Loaded key from keystore");
        println!("SR25519 KEY: {:?}", account_public_key.to_string());
        println!("SR25519 SEED: {:?}", sr25519_keys[0].0);
        println!("SR25519 RAW: {:?}", sr25519_keys);
        Ok(acco_key)
    }
}
