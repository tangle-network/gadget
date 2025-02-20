use bip39::{Language, Mnemonic};
use color_eyre::eyre::Result;
use dialoguer::{Input, Select};
use gadget_crypto::bn254::{ArkBlsBn254Public, ArkBlsBn254Secret};
use gadget_crypto::sp_core::{
    SpBls377, SpBls377Pair, SpBls377Public, SpBls381, SpBls381Pair, SpBls381Public, SpEcdsa,
    SpEcdsaPair, SpEcdsaPublic, SpEd25519, SpEd25519Pair, SpEd25519Public, SpSr25519,
    SpSr25519Pair, SpSr25519Public,
};
use gadget_crypto::{bn254::ArkBlsBn254, KeyTypeId};
use gadget_crypto_core::{KeyEncoding, KeyType};
use gadget_keystore::{backends::Backend, Keystore, KeystoreConfig};
use gadget_std::path::Path;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unknown key type: {0}")]
    UnknownKeyType(String),
    #[error("Keystore error: {0}")]
    KeystoreError(#[from] gadget_keystore::error::Error),
    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),
    #[error("Invalid mnemonic word count: {0}. Must be 12, 15, 18, 21, or 24")]
    InvalidWordCount(u32),
}

pub fn prompt_for_keys(key_types: Vec<KeyTypeId>) -> color_eyre::Result<Vec<(KeyTypeId, String)>> {
    let mut collected_keys = Vec::new();
    let all_key_types = [
        KeyTypeId::Bn254,
        KeyTypeId::Ecdsa,
        KeyTypeId::Sr25519,
        KeyTypeId::Ed25519,
        KeyTypeId::Bls381,
        KeyTypeId::Bls377,
    ];

    if key_types.is_empty() {
        loop {
            let mut options = all_key_types
                .iter()
                .map(|kt| format!("Enter key for {:?}", kt))
                .collect::<Vec<_>>();
            options.push("Continue".to_string());

            let selection = Select::new()
                .with_prompt("Select key type to enter (or Continue when done)")
                .items(&options)
                .default(0)
                .interact()?;

            if selection == options.len() - 1 {
                // User selected "Continue"
                break;
            }

            let key_type = all_key_types[selection];
            let key: String = Input::new()
                .with_prompt(format!("Enter private key for {:?}", key_type))
                .interact_text()?;

            collected_keys.push((key_type, key));
        }
    } else {
        // When specific key types are provided, just prompt for each one
        for key_type in key_types {
            let key: String = Input::new()
                .with_prompt(format!("Enter private key for {:?}", key_type))
                .interact_text()?;

            collected_keys.push((key_type, key));
        }
    }

    Ok(collected_keys)
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
        if let Some(path) = output {
            if !path.as_ref().exists() {
                std::fs::create_dir_all(path.as_ref())?;
            }
        }
        config = config.fs_root(path);
    }

    let keystore = Keystore::new(config)?;

    // Generate key based on type
    let (public_bytes, secret_bytes) = match key_type {
        KeyTypeId::Sr25519 => {
            let public = keystore.generate::<SpSr25519>(seed)?;
            let secret = keystore.get_secret::<SpSr25519>(&public)?;
            keystore.insert::<SpSr25519>(&secret)?;
            (public.to_bytes(), secret.to_bytes())
        }
        KeyTypeId::Ed25519 => {
            let public = keystore.generate::<SpEd25519>(seed)?;
            let secret = keystore.get_secret::<SpEd25519>(&public)?;
            keystore.insert::<SpEd25519>(&secret)?;
            (public.to_bytes(), secret.to_bytes())
        }
        KeyTypeId::Ecdsa => {
            let public = keystore.generate::<SpEcdsa>(seed)?;
            let secret = keystore.get_secret::<SpEcdsa>(&public)?;
            keystore.insert::<SpEcdsa>(&secret)?;
            (public.to_bytes(), secret.to_bytes().to_vec())
        }
        KeyTypeId::Bls381 => {
            let public = keystore.generate::<SpBls381>(seed)?;
            let secret = keystore.get_secret::<SpBls381>(&public)?;
            keystore.insert::<SpBls381>(&secret)?;
            (public.to_bytes(), secret.to_bytes())
        }
        KeyTypeId::Bls377 => {
            let public = keystore.generate::<SpBls377>(seed)?;
            let secret = keystore.get_secret::<SpBls377>(&public)?;
            keystore.insert::<SpBls377>(&secret)?;
            (public.to_bytes(), secret.to_bytes())
        }
        KeyTypeId::Bn254 => {
            let public = keystore.generate::<ArkBlsBn254>(seed)?;
            let secret = keystore.get_secret::<ArkBlsBn254>(&public)?;
            keystore.insert::<ArkBlsBn254>(&secret)?;
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

pub fn generate_mnemonic(word_count: Option<u32>) -> Result<String> {
    let count = match word_count {
        Some(count) if !(12..=24).contains(&count) || count % 3 != 0 => {
            return Err(Error::InvalidWordCount(count).into())
        }
        Some(count) => count,
        None => 12,
    };
    let mut rng = bip39::rand::thread_rng();
    let mnemonic = Mnemonic::generate_in_with(&mut rng, Language::English, count as usize)?;
    Ok(mnemonic.to_string())
}

pub fn import_key(key_type: KeyTypeId, secret: &str, keystore_path: &Path) -> Result<String> {
    let mut config = KeystoreConfig::new();
    config = config.fs_root(keystore_path);
    let keystore = Keystore::new(config)?;

    let secret_bytes = hex::decode(secret).map_err(|e| Error::InvalidKeyFormat(e.to_string()))?;

    let public_key = match key_type {
        KeyTypeId::Sr25519 => {
            let key = SpSr25519Pair::from_bytes(&secret_bytes)?;
            keystore.insert::<SpSr25519>(&key)?;
            hex::encode(key.public().to_bytes())
        }
        KeyTypeId::Ed25519 => {
            let key = SpEd25519Pair::from_bytes(&secret_bytes)?;
            keystore.insert::<SpEd25519>(&key)?;
            hex::encode(key.public().to_bytes())
        }
        KeyTypeId::Ecdsa => {
            let key = SpEcdsaPair::from_bytes(&secret_bytes)?;
            keystore.insert::<SpEcdsa>(&key)?;
            hex::encode(key.public().to_bytes())
        }
        KeyTypeId::Bls381 => {
            let key = SpBls381Pair::from_bytes(&secret_bytes)?;
            keystore.insert::<SpBls381>(&key)?;
            hex::encode(key.public().to_bytes())
        }
        KeyTypeId::Bls377 => {
            let key = SpBls377Pair::from_bytes(&secret_bytes)?;
            keystore.insert::<SpBls377>(&key)?;
            hex::encode(key.public().to_bytes())
        }
        KeyTypeId::Bn254 => {
            let key = ArkBlsBn254Secret::from_bytes(&secret_bytes)?;
            keystore.insert::<ArkBlsBn254>(&key)?;
            let public = ArkBlsBn254::public_from_secret(&key);
            hex::encode(public.to_bytes())
        }
    };

    Ok(public_key)
}

pub fn export_key(key_type: KeyTypeId, public: &str, keystore_path: &Path) -> Result<String> {
    let mut config = KeystoreConfig::new();
    config = config.fs_root(keystore_path);
    let keystore = Keystore::new(config)?;

    let public_bytes = hex::decode(public).map_err(|e| Error::InvalidKeyFormat(e.to_string()))?;

    let secret = match key_type {
        KeyTypeId::Sr25519 => {
            let public = SpSr25519Public::from_bytes(&public_bytes)?;
            let secret = keystore.get_secret::<SpSr25519>(&public)?;
            hex::encode(secret.to_bytes())
        }
        KeyTypeId::Ed25519 => {
            let public = SpEd25519Public::from_bytes(&public_bytes)?;
            let secret = keystore.get_secret::<SpEd25519>(&public)?;
            hex::encode(secret.to_bytes())
        }
        KeyTypeId::Ecdsa => {
            let public = SpEcdsaPublic::from_bytes(&public_bytes)?;
            let secret = keystore.get_secret::<SpEcdsa>(&public)?;
            hex::encode(secret.to_bytes())
        }
        KeyTypeId::Bls381 => {
            let public = SpBls381Public::from_bytes(&public_bytes)?;
            let secret = keystore.get_secret::<SpBls381>(&public)?;
            hex::encode(secret.to_bytes())
        }
        KeyTypeId::Bls377 => {
            let public = SpBls377Public::from_bytes(&public_bytes)?;
            let secret = keystore.get_secret::<SpBls377>(&public)?;
            hex::encode(secret.to_bytes())
        }
        KeyTypeId::Bn254 => {
            let public = ArkBlsBn254Public::from_bytes(&public_bytes)?;
            let secret = keystore.get_secret::<ArkBlsBn254>(&public)?;
            hex::encode(secret.to_bytes())
        }
    };

    Ok(secret)
}

pub fn list_keys(keystore_path: &Path) -> Result<Vec<(KeyTypeId, String)>> {
    let mut config = KeystoreConfig::new();
    config = config.fs_root(keystore_path);
    let keystore = Keystore::new(config)?;

    let mut keys = Vec::new();

    // List keys for each type
    for key in keystore.list_local::<SpSr25519>()? {
        keys.push((KeyTypeId::Sr25519, hex::encode(key.to_bytes())));
    }
    for key in keystore.list_local::<SpEd25519>()? {
        keys.push((KeyTypeId::Ed25519, hex::encode(key.to_bytes())));
    }
    for key in keystore.list_local::<SpEcdsa>()? {
        keys.push((KeyTypeId::Ecdsa, hex::encode(key.to_bytes())));
    }
    for key in keystore.list_local::<SpBls381>()? {
        keys.push((KeyTypeId::Bls381, hex::encode(key.to_bytes())));
    }
    for key in keystore.list_local::<SpBls377>()? {
        keys.push((KeyTypeId::Bls377, hex::encode(key.to_bytes())));
    }
    for key in keystore.list_local::<ArkBlsBn254>()? {
        keys.push((KeyTypeId::Bn254, hex::encode(key.to_bytes())));
    }

    Ok(keys)
}
