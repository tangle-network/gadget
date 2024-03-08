use color_eyre::Result;
use sc_keystore::LocalKeystore;
use sp_keystore::KeystorePtr;
use std::sync::Arc;

use crate::config::KeystoreConfig;

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
    pub fn keystore(&self) -> KeystorePtr {
        self.0.clone()
    }

    /// Returns a shared reference to the local keystore .
    pub fn local_keystore(&self) -> Arc<LocalKeystore> {
        self.0.clone()
    }
}
