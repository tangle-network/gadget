use color_eyre::Result;
use sp_keystore::{Error, KeystorePtr, Keystore};
use sp_core::{
    crypto::{ByteArray, KeyTypeId, Pair, VrfSecret},
    ecdsa, ed25519, sr25519,
};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use crate::config::KeystoreConfig;

use sp_application_crypto::AppPair;

/// Construct a local keystore shareable container
pub struct KeystoreContainer(Arc<MemoryKeystore>);

impl KeystoreContainer {
    /// Construct KeystoreContainer
    pub fn new(config: &KeystoreConfig) -> Result<Self> {
        let keystore = Arc::new(
            MemoryKeystore::new()
            // match config {
            //     KeystoreConfig::Path { path, password } => {
            //         LocalKeystore::open(path.clone(), password.clone())?
            //     }
            //     KeystoreConfig::InMemory => LocalKeystore::in_memory(),
            // }
        );

        Ok(Self(keystore))
    }

    /// Returns a shared reference to a dynamic `Keystore` trait implementation.
    #[allow(dead_code)]
    pub fn keystore(&self) -> KeystorePtr {
        self.0.clone()
    }

    /// Returns a shared reference to the local keystore .
    pub fn local_keystore(&self) -> Arc<MemoryKeystore> {
        self.0.clone()
    }
}
