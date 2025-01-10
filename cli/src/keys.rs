use color_eyre::eyre::Result;
use gadget_crypto::sp_core::{SpBls377, SpBls381, SpEcdsa, SpEd25519, SpSr25519};
use gadget_crypto::{bn254::ArkBlsBn254, KeyTypeId};
use gadget_crypto_core::KeyEncoding;
use gadget_keystore::{backends::Backend, Keystore, KeystoreConfig};
use std::path::Path;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unknown key type: {0}")]
    UnknownKeyType(String),
    #[error("Keystore error: {0}")]
    KeystoreError(#[from] gadget_keystore::error::Error),
}

pub fn generate_key(
    key_type: KeyTypeId,
    output: Option<&impl AsRef<Path>>,
    seed: Option<&[u8]>,
    show_secret: bool,
) -> Result<(String, Option<String>)> {
    // Create keystore configuration
    let mut config = KeystoreConfig::new();
    if let Some(path) = output {
        config = config.fs_root(path);
    }

    let keystore = Keystore::new(config)?;

    // Generate key based on type
    let (public_bytes, secret_bytes) = match key_type {
        KeyTypeId::Sr25519 => {
            let public = keystore.generate::<SpSr25519>(seed)?;
            let secret = keystore.get_secret::<SpSr25519>(&public)?;
            (public.to_bytes(), secret.to_bytes())
        }
        KeyTypeId::Ed25519 => {
            let public = keystore.generate::<SpEd25519>(seed)?;
            let secret = keystore.get_secret::<SpEd25519>(&public)?;
            (public.to_bytes(), secret.to_bytes())
        }
        KeyTypeId::Ecdsa => {
            let public = keystore.generate::<SpEcdsa>(seed)?;
            let secret = keystore.get_secret::<SpEcdsa>(&public)?;
            (public.to_bytes(), secret.to_bytes().to_vec())
        }
        KeyTypeId::Bls381 => {
            let public = keystore.generate::<SpBls381>(seed)?;
            let secret = keystore.get_secret::<SpBls381>(&public)?;
            (public.to_bytes(), secret.to_bytes())
        }
        KeyTypeId::Bls377 => {
            let public = keystore.generate::<SpBls377>(seed)?;
            let secret = keystore.get_secret::<SpBls377>(&public)?;
            (public.to_bytes(), secret.to_bytes())
        }
        KeyTypeId::Bn254 => {
            let public = keystore.generate::<ArkBlsBn254>(seed)?;
            let secret = keystore.get_secret::<ArkBlsBn254>(&public)?;
            (public.to_bytes(), secret.to_bytes())
        }
    };

    let (public, secret) = (hex::encode(public_bytes), hex::encode(secret_bytes));

    let mut secret = Some(secret);
    if !show_secret {
        secret = None;
    }

    Ok((public, secret))
}
