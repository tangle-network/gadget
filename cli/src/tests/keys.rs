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

#[test]
fn test_generate_mnemonic() -> Result<()> {
    use crate::keys::generate_mnemonic;

    // Test default word count
    let mnemonic = generate_mnemonic(None)?;
    let words: Vec<&str> = mnemonic.split_whitespace().collect();
    assert_eq!(words.len(), 12); // Default should be 12 words

    // Test custom word count
    let word_counts = [12, 15, 18, 21, 24];
    for count in word_counts {
        let mnemonic = generate_mnemonic(Some(count))?;
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), count as usize);
    }

    Ok(())
}

#[test]
fn test_key_import_export() -> Result<()> {
    use crate::keys::{export_key, import_key};
    let temp_dir = tempdir()?;
    let keystore_path = temp_dir.path();

    // Test key import and export for each key type
    for key_type in [
        KeyTypeId::Sr25519,
        KeyTypeId::Ed25519,
        KeyTypeId::Ecdsa,
        KeyTypeId::Bls381,
        KeyTypeId::Bn254,
    ] {
        // First generate a key to get a valid secret
        let (_public, secret) = generate_key(key_type, Some(&keystore_path), None, true)?;
        let secret = secret.unwrap();

        // Import the key
        let imported_public = import_key(key_type, &secret, keystore_path)?;
        assert!(!imported_public.is_empty());

        // Export the key
        let exported_secret = export_key(key_type, &imported_public, keystore_path)?;
        assert!(!exported_secret.is_empty());

        // Make sure the secrets match
        assert_eq!(secret, exported_secret);
    }

    Ok(())
}

#[test]
fn test_list_keys() -> Result<()> {
    use crate::keys::list_keys;
    let temp_dir = tempdir()?;
    let keystore_path = temp_dir.path();

    // Generate some keys first
    let test_types = [KeyTypeId::Sr25519, KeyTypeId::Ed25519, KeyTypeId::Ecdsa];

    let mut expected_keys = Vec::new();
    for key_type in test_types {
        let (public, _) = generate_key(key_type, Some(&keystore_path), None, true)?;
        expected_keys.push((key_type, public));
    }

    // List keys and verify
    let listed_keys = list_keys(keystore_path)?;
    assert_eq!(listed_keys.len(), expected_keys.len());

    // Verify each expected key is in the listed keys
    for expected in expected_keys {
        assert!(listed_keys
            .iter()
            .any(|k| k.0 == expected.0 && k.1 == expected.1));
    }

    Ok(())
}
