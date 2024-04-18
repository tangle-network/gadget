pub use crate::shared::keystore::SubstrateKeystore;

use tracing;
use color_eyre;
use std::path::{PathBuf, Path};
use sp_core::{ecdsa, ed25519, sr25519, ByteArray, Pair, crypto};
use color_eyre::eyre::OptionExt;

use k256::ecdsa::{SigningKey as SecretKey, VerifyingKey};

use color_eyre::Result;
// use sp_keystore::{Error, KeystorePtr, Keystore};
// use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use anyhow::Error;

// use sp_application_crypto::AppPair;

use crate::log;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures;
use wasm_bindgen::JsValue;
use crate::into_js_error as into_js_error;


/// Construct a local keystore shareable container
pub struct KeystoreContainer(Arc<String>);

impl KeystoreContainer {
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
}

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
    InMemory {
        keystore: Vec<String>,
    },
}

impl KeystoreConfig {
    /// Returns the path for the keystore.
    #[allow(dead_code)]
    pub fn path(&self) -> Option<&Path> {
        // match self {
        //     // Self::Path { path, .. } => Some(path),
        //     Self::InMemory => None,
        // }
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
            KeystoreConfig::InMemory { keystore: keys } => {
                keys
            }
            _ => {
                panic!("This should never happen");
            }
        };
        log(&format!("ECDSA KEY LIST: {:?}", keys));
        let key = hex::decode(keys[0].clone())?;
        let ecdsa_seed: &[u8] = key.as_slice().try_into()?;
        // let ecdsa_key = ecdsa::Pair::from_seed_slice(keys.as_bytes()).map_err(|e| Error::msg(format!("{:?}", e))).unwrap();
        let ecdsa_key = ecdsa::Pair::from_seed_slice(ecdsa_seed.clone()).map_err(|e| color_eyre::eyre::eyre!("{e:?}"))?;

            // .into_inner();
            // .map_err(Error::msg)?;
        let supposed_public_key = VerifyingKey::from(SecretKey::from_slice(ecdsa_seed)?);
        log(&format!("SUPPOSED ECDSA PUBLIC KEY - VERIFYING?: {:?}", supposed_public_key));

        log(&format!("ECDSA KEY PAIR?: {:?}", ecdsa_key.to_raw_vec()));
        log(&format!("ECDSA PUBLIC KEY?: {:?}", ecdsa_key.public()));
        log(&format!("ECDSA SEED?: {:?}", ecdsa_key.seed()));

        Ok(ecdsa_key)
        // let role_public_key = crypto::role::Public::from_slice(&ecdsa_key)
        //     .map_err(into_js_error)?;
        //
        // let role_key = keystore
        //     .key_pair::<crypto::role::Pair>(&role_public_key)
        //     .map_err(into_js_error)?
        //     .into_inner();

        // let keystore_container = KeystoreContainer::new(self)?;
        // let keystore = keystore_container.local_keystore();
        // tracing::debug!("Loaded keystore from path");
        // let ecdsa_keys = keystore.ecdsa_public_keys(crypto::role::KEY_TYPE);
        //
        // if ecdsa_keys.len() != 1 {
        //     color_eyre::eyre::bail!(
		// 		"`role`: Expected exactly one key in ECDSA keystore, found {}",
		// 		ecdsa_keys.len()
		// 	);
        // }
        //
        // let role_public_key = crypto::role::Public::from_slice(&ecdsa_keys[0].0)
        //     .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;
        //
        // let role_key = keystore
        //     .key_pair::<crypto::role::Pair>(&role_public_key)?
        //     .ok_or_eyre("Failed to load key `role` from keystore")?
        //     .into_inner();
        //
        //
        // tracing::debug!(%role_public_key, "Loaded key from keystore");
        //
        // Ok(role_key)
    }

    fn sr25519_key(&self) -> color_eyre::Result<sr25519::Pair> {
        let keys = match self {
            KeystoreConfig::InMemory { keystore: keys } => {
                keys
            }
            _ => {
                panic!("This should never happen");
            }
        };
        log(&format!("SR25519 KEY LIST: {:?}", keys));
        // let sr25519_key = sr25519::Pair::from_seed_slice(keys.as_bytes()).map_err(|e| Error::msg(format!("{:?}", e))).unwrap();
        let key = hex::decode(keys[1].clone())?;
        let sr25519_key = key.as_slice().try_into()?;

        let sr25519_key = sr25519::Pair::from_seed_slice(sr25519_key).map_err(|e| color_eyre::eyre::eyre!("{e:?}"))?;
            // .into_inner();
            // .map_err(Error::msg)?;
        log(&format!("SR25519 KEY PAIR?: {:?}", sr25519_key.to_raw_vec()));
        Ok(sr25519_key)

        // let keystore_container = KeystoreContainer::new(self)?;
        // let keystore = keystore_container.local_keystore();
        // tracing::debug!("Loaded keystore from path");
        // let sr25519_keys = keystore.sr25519_public_keys(crypto::acco::KEY_TYPE);
        //
        // if sr25519_keys.len() != 1 {
        //     color_eyre::eyre::bail!(
        //         "`acco`: Expected exactly one key in SR25519 keystore, found {}",
        //         sr25519_keys.len()
        //     );
        // }
        //
        // let account_public_key = crypto::acco::Public::from_slice(&sr25519_keys[0].0)
        //     .map_err(|_| color_eyre::eyre::eyre!("Failed to parse public key from keystore"))?;
        //
        // let acco_key = keystore
        //     .key_pair::<crypto::acco::Pair>(&account_public_key)?
        //     .ok_or_eyre("Failed to load key `acco` from keystore")?
        //     .into_inner();
        //
        // tracing::debug!(%account_public_key, "Loaded key from keystore");
        // Ok(acco_key)
    }
}