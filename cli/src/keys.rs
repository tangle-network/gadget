use color_eyre::eyre::Result;
use gadget_crypto::{
    bls_crypto::{bls377::W3fBls377, bls381::W3fBls381},
    bn254_crypto::ArkBlsBn254,
    ed25519_crypto::Ed25519Zebra,
    k256_crypto::K256Ecdsa,
    sp_core_crypto::{SpBls377, SpBls381, SpEcdsa, SpEd25519, SpSr25519},
    sr25519_crypto::SchnorrkelSr25519,
    KeyTypeId,
};
use gadget_keystore::{backends::Backend, Keystore, KeystoreConfig};
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unknown key type: {0}")]
    UnknownKeyType(String),
    #[error("Keystore error: {0}")]
    KeystoreError(#[from] gadget_keystore::error::Error),
}

pub fn generate_key(
    key_type: KeyTypeId,
    output: Option<PathBuf>,
    seed: Option<&[u8]>,
    show_secret: bool,
) -> Result<()> {
    // Create keystore configuration
    let mut config = KeystoreConfig::new();
    if let Some(ref path) = output {
        config = config.fs_root(path);
    }

    let keystore = Keystore::new(config)?;

    // Generate key based on type
    let (public_bytes, secret_bytes) = match key_type {
        KeyTypeId::SchnorrkelSr25519 => {
            let public = keystore.generate::<SchnorrkelSr25519>(seed)?;
            let secret = keystore.get_secret::<SchnorrkelSr25519>(&public)?;
            (serde_json::to_vec(&public)?, serde_json::to_vec(&secret)?)
        }
        KeyTypeId::ZebraEd25519 => {
            let public = keystore.generate::<Ed25519Zebra>(seed)?;
            let secret = keystore.get_secret::<Ed25519Zebra>(&public)?;
            (serde_json::to_vec(&public)?, serde_json::to_vec(&secret)?)
        }
        KeyTypeId::K256Ecdsa => {
            let public = keystore.generate::<K256Ecdsa>(seed)?;
            let secret = keystore.get_secret::<K256Ecdsa>(&public)?;
            (serde_json::to_vec(&public)?, serde_json::to_vec(&secret)?)
        }
        KeyTypeId::W3fBls381 => {
            let public = keystore.generate::<W3fBls381>(seed)?;
            let secret = keystore.get_secret::<W3fBls381>(&public)?;
            (serde_json::to_vec(&public)?, serde_json::to_vec(&secret)?)
        }
        KeyTypeId::W3fBls377 => {
            let public = keystore.generate::<W3fBls377>(seed)?;
            let secret = keystore.get_secret::<W3fBls377>(&public)?;
            (serde_json::to_vec(&public)?, serde_json::to_vec(&secret)?)
        }
        KeyTypeId::ArkBn254 => {
            let public = keystore.generate::<ArkBlsBn254>(seed)?;
            let secret = keystore.get_secret::<ArkBlsBn254>(&public)?;
            (serde_json::to_vec(&public)?, serde_json::to_vec(&secret)?)
        }
        KeyTypeId::SpEcdsa => {
            let public = keystore.generate::<SpEcdsa>(seed)?;
            let secret = keystore.get_secret::<SpEcdsa>(&public)?;
            (serde_json::to_vec(&public)?, serde_json::to_vec(&secret)?)
        }
        KeyTypeId::SpEd25519 => {
            let public = keystore.generate::<SpEd25519>(seed)?;
            let secret = keystore.get_secret::<SpEd25519>(&public)?;
            (serde_json::to_vec(&public)?, serde_json::to_vec(&secret)?)
        }
        KeyTypeId::SpSr25519 => {
            let public = keystore.generate::<SpSr25519>(seed)?;
            let secret = keystore.get_secret::<SpSr25519>(&public)?;
            (serde_json::to_vec(&public)?, serde_json::to_vec(&secret)?)
        }
        KeyTypeId::SpBls377 => {
            let public = keystore.generate::<SpBls377>(seed)?;
            let secret = keystore.get_secret::<SpBls377>(&public)?;
            (serde_json::to_vec(&public)?, serde_json::to_vec(&secret)?)
        }
        KeyTypeId::SpBls381 => {
            let public = keystore.generate::<SpBls381>(seed)?;
            let secret = keystore.get_secret::<SpBls381>(&public)?;
            (serde_json::to_vec(&public)?, serde_json::to_vec(&secret)?)
        }
    };

    let (public, secret) = (hex::encode(public_bytes), hex::encode(secret_bytes));

    eprintln!("Generated {} key:", key_type.name());
    eprintln!("Public key: {}", public);
    if show_secret || output.is_none() {
        eprintln!("Private key: {}", secret);
    }

    Ok(())
}
