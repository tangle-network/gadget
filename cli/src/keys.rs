use color_eyre::eyre::eyre;
use gadget_sdk::keystore::backend::fs::FilesystemKeystore;
use gadget_sdk::keystore::backend::mem::InMemoryKeystore;
use gadget_sdk::keystore::backend::GenericKeyStore;
use gadget_sdk::keystore::Backend;
use gadget_sdk::parking_lot::RawRwLock;
use std::path::PathBuf;
use w3f_bls::serialize::SerializableToBytes;

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum KeyType {
    Sr25519,
    Ed25519,
    Ecdsa,
    Bls381,
    BlsBn254,
}

pub fn generate_key(
    key_type: KeyType,
    output: Option<PathBuf>,
    seed: Option<&[u8]>,
    show_secret: bool,
) -> color_eyre::Result<()> {
    let keystore: GenericKeyStore<RawRwLock> = match output {
        None => GenericKeyStore::Mem(InMemoryKeystore::new()),
        Some(file_path) => {
            // Filesystem Keystore
            GenericKeyStore::Fs(
                FilesystemKeystore::open(file_path).map_err(|e| eyre!(e.to_string()))?,
            )
        }
    };

    let (public, secret) = match key_type {
        KeyType::Sr25519 => {
            let public_key = keystore
                .sr25519_generate_new(seed)
                .map_err(|e| eyre!(e.to_string()))?;
            let secret = keystore
                .expose_sr25519_secret(&public_key)
                .map_err(|e| eyre!(e.to_string()))?
                .ok_or(eyre!("Failed to expose secret"))?;
            (
                hex::encode(public_key.to_bytes()),
                hex::encode(secret.to_bytes()),
            )
        }
        KeyType::Ed25519 => {
            let public_key = keystore
                .ed25519_generate_new(seed)
                .map_err(|e| eyre!(e.to_string()))?;
            let secret = keystore
                .expose_ed25519_secret(&public_key)
                .map_err(|e| eyre!(e.to_string()))?
                .ok_or(eyre!("Failed to expose secret"))?;
            (hex::encode(public_key), hex::encode(secret))
        }
        KeyType::Ecdsa => {
            let public_key = keystore
                .ecdsa_generate_new(seed)
                .map_err(|e| eyre!(e.to_string()))?;
            let secret = keystore
                .expose_ecdsa_secret(&public_key)
                .map_err(|e| eyre!(e.to_string()))?
                .ok_or(eyre!("Failed to expose secret"))?;
            (
                hex::encode(public_key.to_sec1_bytes()),
                hex::encode(secret.to_bytes()),
            )
        }
        KeyType::Bls381 => {
            let public_key = keystore
                .bls381_generate_new(seed)
                .map_err(|e| eyre!(e.to_string()))?;
            let secret = keystore
                .expose_bls381_secret(&public_key)
                .map_err(|e| eyre!(e.to_string()))?
                .ok_or(eyre!("Failed to expose secret"))?;
            (
                hex::encode(public_key.0.to_string()),
                hex::encode(secret.to_bytes()),
            )
        }
        KeyType::BlsBn254 => {
            let public_key = keystore
                .bls_bn254_generate_new(seed)
                .map_err(|e| eyre!(e.to_string()))?;
            let secret = keystore
                .expose_bls_bn254_secret(&public_key)
                .map_err(|e| eyre!(e.to_string()))?
                .ok_or(eyre!("Failed to expose secret"))?;
            (
                hex::encode(public_key.g1().to_string()),
                hex::encode(secret.0.to_string()),
            )
        }
    };

    println!("Generated {:?} key:", key_type);
    println!("Public key: {}", public);
    if show_secret {
        println!("Private Key: {}", secret);
    }

    Ok(())
}
