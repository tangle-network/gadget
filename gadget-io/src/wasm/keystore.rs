pub use crate::shared::keystore::SubstrateKeystore;

use color_eyre;
use sp_core::{ecdsa, sr25519, Pair};
use std::path::Path;

// /// Construct a local keystore shareable container
pub struct KeystoreContainer;

// impl KeystoreContainer {
// /// Construct KeystoreContainer
// pub fn new() -> Result<Self> {
//     let keystore = Arc::new(WasmKeystore);
//     Ok(Self(keystore))
// }
//
// /// Returns a shared reference to a dynamic `Keystore` trait implementation.
// #[allow(dead_code)]
// pub fn keystore(&self) -> KeystorePtr {
//     self.0.clone()
// }
//
// /// Returns a shared reference to the local keystore .
// pub fn local_keystore(&self) -> Arc<WasmKeystore> {
//     self.0.clone()
// }
// }

/// Configuration of the client keystore.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum KeystoreConfig {
    // /// Keystore at a path on-disk. Recommended for native gadgets.
    // Path {
    //     /// The path of the keystore.
    //     path: PathBuf,
    //     /// keystore's password.
    //     password: Option<SecretString>,
    // },
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

// #[wasm_bindgen]
// extern "C" {
//     async fn get_ecdsa_keys();
//     async fn get_sr25519_keys();
// }

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
