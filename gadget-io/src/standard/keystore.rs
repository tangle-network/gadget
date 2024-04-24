pub use crate::shared::keystore::SubstrateKeystore;
use tracing;
use color_eyre;
use std::path::{PathBuf, Path};
use sp_core::{ecdsa, ed25519, sr25519, ByteArray, Pair};
use color_eyre::eyre::OptionExt;

use color_eyre::Result;
use sc_keystore::{ LocalKeystore, Keystore };
use sp_keystore::KeystorePtr;
use std::sync::Arc;


/// Construct a local keystore shareable container
pub struct KeystoreContainer(Arc<LocalKeystore>);

impl KeystoreContainer {
    /// Construct KeystoreContainer
    pub fn new(config: &KeystoreConfig) -> Result<Self> {
        let keystore = Arc::new(match config {
            KeystoreConfig::Path { path, password } => {
                LocalKeystore::open(path.clone(), password.clone())?
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
    fn ecdsa_key(&self) -> color_eyre::Result<ecdsa::Pair> {
        let keystore_container = KeystoreContainer::new(self)?;
        let keystore = keystore_container.local_keystore();
        tracing::debug!("Loaded keystore from path");
        let ecdsa_keys = keystore.ecdsa_public_keys(crypto::role::KEY_TYPE);

        if ecdsa_keys.len() != 1 {
            color_eyre::eyre::bail!(
				"`role`: Expected exactly one key in ECDSA keystore, found {}",
				ecdsa_keys.len()
			);
        }

        let role_public_key = crypto::role::Public::from_slice(&ecdsa_keys[0].0)
            .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;

        let role_key = keystore
            .key_pair::<crypto::role::Pair>(&role_public_key)?
            .ok_or_eyre("Failed to load key `role` from keystore")?
            .into_inner();


        tracing::debug!(%role_public_key, "Loaded key from keystore");

        Ok(role_key)
    }

    fn sr25519_key(&self) -> color_eyre::Result<sr25519::Pair> {
        let keystore_container = KeystoreContainer::new(self)?;
        let keystore = keystore_container.local_keystore();
        tracing::debug!("Loaded keystore from path");
        let sr25519_keys = keystore.sr25519_public_keys(crypto::acco::KEY_TYPE);

        if sr25519_keys.len() != 1 {
            color_eyre::eyre::bail!(
                "`acco`: Expected exactly one key in SR25519 keystore, found {}",
                sr25519_keys.len()
            );
        }

        let account_public_key = crypto::acco::Public::from_slice(&sr25519_keys[0].0)
            .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;

        let acco_key = keystore
            .key_pair::<crypto::acco::Pair>(&account_public_key)?
            .ok_or_eyre("Failed to load key `acco` from keystore")?
            .into_inner();

        tracing::debug!(%account_public_key, "Loaded key from keystore");
        Ok(acco_key)
    }
}