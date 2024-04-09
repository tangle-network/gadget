pub use crate::shared::keystore::{KeystoreConfig, SubstrateKeystore};
use tracing;
use color_eyre;
use std::path::{PathBuf, Path};
use sp_core::{ecdsa, ed25519, sr25519, ByteArray, Pair, crypto};


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