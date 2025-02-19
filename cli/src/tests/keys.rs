use crate::keys::generate_key;
use color_eyre::eyre::Result;
use gadget_crypto::bn254::ArkBlsBn254;
use gadget_crypto::sp_core::{SpBls381, SpEcdsa, SpEd25519, SpSr25519};
use gadget_crypto_core::KeyTypeId;
use gadget_keystore::{backends::Backend, Keystore, KeystoreConfig};
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_cli_fs_key_generation() -> Result<()> {
    let temp_dir = tempdir()?;
    let output_path = temp_dir.path();

    for key_type in [
        KeyTypeId::Sr25519,
        KeyTypeId::Ed25519,
        KeyTypeId::Ecdsa,
        KeyTypeId::Bls381,
        KeyTypeId::Bn254,
    ]
    .iter()
    {
        println!("Testing key generation for: {:?}", key_type);
        let (public, secret) = generate_key(*key_type, Some(&output_path), None, true)?;
        assert!(!public.is_empty());
        assert!(secret.is_some());
        assert!(!secret.unwrap().is_empty());

        let keystore = Keystore::new(KeystoreConfig::new().fs_root(output_path))?;
        match key_type {
            KeyTypeId::Sr25519 => {
                keystore.first_local::<SpSr25519>()?;
            }
            KeyTypeId::Ed25519 => {
                keystore.first_local::<SpEd25519>()?;
            }
            KeyTypeId::Ecdsa => {
                keystore.first_local::<SpEcdsa>()?;
            }
            KeyTypeId::Bls381 => {
                keystore.first_local::<SpBls381>()?;
            }
            KeyTypeId::Bn254 => {
                keystore.first_local::<ArkBlsBn254>()?;
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}

#[test]
fn test_cli_mem_key_generation() -> Result<()> {
    for key_type in [
        KeyTypeId::Sr25519,
        KeyTypeId::Ed25519,
        KeyTypeId::Ecdsa,
        KeyTypeId::Bls381,
        KeyTypeId::Bn254,
    ]
    .iter()
    {
        println!("Testing key generation for: {:?}", key_type);
        let (public, secret) = generate_key(*key_type, None::<&PathBuf>, None, true)?;
        assert!(!public.is_empty());
        assert!(secret.is_some());
        assert!(!secret.unwrap().is_empty());
    }
    Ok(())
}
