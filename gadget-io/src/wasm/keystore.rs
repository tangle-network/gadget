pub use crate::shared::keystore::SubstrateKeystore;

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures;
use tracing;
use color_eyre;
use std::path::{PathBuf, Path};
use sp_core::{ecdsa, ed25519, sr25519, ByteArray, Pair, crypto};
use wasm_bindgen::JsValue;

#[wasm_bindgen(module = "/js-utils.js")]
extern "C" {
    async fn fileTest();
    //pub fn fileTest() -> JsValue;
    // fn helloWorld();
}

pub async fn file_test() {
    fileTest().await;
}

// pub fn hello_world() {
//     helloWorld();
// }

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Configuration of the client keystore.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum KeystoreConfig {
    /// Keystore at a path on-disk. Recommended for native gadgets.
    // Path {
    //     /// The path of the keystore.
    //     path: PathBuf,
    //     /// keystore's password.
    //     password: Option<SecretString>,
    // },
    /// In-memory keystore.
    InMemory,
}

impl KeystoreConfig {
    /// Returns the path for the keystore.
    #[allow(dead_code)]
    pub fn path(&self) -> Option<&Path> {
        match self {
            // Self::Path { path, .. } => Some(path),
            Self::InMemory => None,
        }
    }
}

// #[wasm_bindgen]
// extern "C" {
//     async fn get_ecdsa_keys();
//     async fn get_sr25519_keys();
// }

// impl SubstrateKeystore for KeystoreConfig {
//     fn ecdsa_key(&self) -> color_eyre::Result<ecdsa::Pair> {
//         let keystore_container = KeystoreContainer::new(self)?;
//         let keystore = keystore_container.local_keystore();
//         tracing::debug!("Loaded keystore from path");
//         let ecdsa_keys = keystore.ecdsa_public_keys(crypto::role::KEY_TYPE);
//
//         if ecdsa_keys.len() != 1 {
//             color_eyre::eyre::bail!(
// 				"`role`: Expected exactly one key in ECDSA keystore, found {}",
// 				ecdsa_keys.len()
// 			);
//         }
//
//         let role_public_key = crypto::role::Public::from_slice(&ecdsa_keys[0].0)
//             .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;
//
//         let role_key = keystore
//             .key_pair::<crypto::role::Pair>(&role_public_key)?
//             .ok_or_eyre("Failed to load key `role` from keystore")?
//             .into_inner();
//
//
//         tracing::debug!(%role_public_key, "Loaded key from keystore");
//
//         Ok(role_key)
//     }
//
//     fn sr25519_key(&self) -> color_eyre::Result<sr25519::Pair> {
//         let keystore_container = KeystoreContainer::new(self)?;
//         let keystore = keystore_container.local_keystore();
//         tracing::debug!("Loaded keystore from path");
//         let sr25519_keys = keystore.sr25519_public_keys(crypto::acco::KEY_TYPE);
//
//         if sr25519_keys.len() != 1 {
//             color_eyre::eyre::bail!(
//                 "`acco`: Expected exactly one key in SR25519 keystore, found {}",
//                 sr25519_keys.len()
//             );
//         }
//
//         let account_public_key = crypto::acco::Public::from_slice(&sr25519_keys[0].0)
//             .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;
//
//         let acco_key = keystore
//             .key_pair::<crypto::acco::Pair>(&account_public_key)?
//             .ok_or_eyre("Failed to load key `acco` from keystore")?
//             .into_inner();
//
//         tracing::debug!(%account_public_key, "Loaded key from keystore");
//         Ok(acco_key)
//     }
// }