use crate::keys::KeyType;
use crate::signer::{load_evm_signer_from_env, load_signer_from_env, EVM_SIGNER_ENV, SIGNER_ENV};
use color_eyre::eyre::Result;
use gadget_sdk::ext::sp_core::Pair;
use gadget_sdk::keystore::{BackendExt, KeystoreUriSanitizer};
use std::env;
use std::process::Command;
use tangle_subxt::subxt_signer::bip39;
use tempfile::tempdir;
use w3f_bls::SerializableToBytes;

#[test]
fn test_cli_fs_key_generation() -> Result<()> {
    let temp_dir = tempdir()?;
    let output_path = temp_dir.path().sanitize_file_path();
    let keystore =
        gadget_sdk::keystore::backend::fs::FilesystemKeystore::open(output_path.clone())?;

    for key_type in [
        KeyType::Sr25519,
        KeyType::Ed25519,
        KeyType::Ecdsa,
        KeyType::Bls381,
        KeyType::BlsBn254,
    ]
    .iter()
    {
        println!("Testing key generation for: {:?}", key_type);
        let mut cmd = Command::new("./../target/release/cargo-tangle");
        let key_str = format!("{:?}", key_type).to_lowercase();

        let output = cmd
            .arg("blueprint")
            .arg("keygen")
            .arg("-k")
            .arg(key_str)
            .arg("-p")
            .arg(output_path.clone())
            .arg("--show-secret")
            .output()?;

        assert!(&output.stderr.is_empty());

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout_str.contains("Public key:"),
            "Public key not found in output for {:?}",
            key_type
        );
        assert!(
            stdout_str.contains("Private key:"),
            "Secret key not found in output for {:?}",
            key_type
        );

        match key_type {
            KeyType::Sr25519 => {
                let pair = keystore.sr25519_key()?;
                assert!(!pair.public().is_empty());
            }
            KeyType::Ed25519 => {
                let pair = keystore.ed25519_key()?;
                assert!(!pair.public().is_empty());
            }
            KeyType::Ecdsa => {
                let pair = keystore.ecdsa_key()?;
                assert!(!pair.public().0.is_empty());
            }
            KeyType::Bls381 => {
                let pair = keystore.bls381_key()?;
                assert!(!pair.public.to_bytes().is_empty());
            }
            KeyType::BlsBn254 => {
                let pair = keystore.bls_bn254_key()?;
                assert!(!pair.public_key().g1().to_string().is_empty());
            }
        }
    }
    Ok(())
}

#[test]
fn test_cli_mem_key_generation() -> Result<()> {
    for key_type in [
        KeyType::Sr25519,
        KeyType::Ed25519,
        KeyType::Ecdsa,
        KeyType::Bls381,
        KeyType::BlsBn254,
    ]
    .iter()
    {
        println!("Testing key generation for: {:?}", key_type);
        let mut cmd = Command::new("./../target/release/cargo-tangle");
        let key_str = format!("{:?}", key_type).to_lowercase();

        let output = cmd
            .arg("blueprint")
            .arg("keygen")
            .arg("-k")
            .arg(key_str)
            .arg("--show-secret")
            .output()?;

        assert!(&output.stderr.is_empty());

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout_str.contains("Public key:"),
            "Public key not found in output for {:?}",
            key_type
        );
        assert!(
            stdout_str.contains("Private key:"),
            "Secret key not found in output for {:?}",
            key_type
        );
    }
    Ok(())
}

#[test]
fn test_load_signer_from_env() -> color_eyre::Result<()> {
    color_eyre::install().unwrap_or(());
    let s = [1u8; 32];
    let secret = bip39::Mnemonic::from_entropy(&s[..])?.to_string();
    // Test with a valid mnemonic phrase
    env::set_var(SIGNER_ENV, secret);
    load_signer_from_env()?;

    // Test with a valid hex string
    env::set_var(
        SIGNER_ENV,
        "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    load_signer_from_env()?;

    // Test with an invalid mnemonic phrase
    env::set_var(SIGNER_ENV, "invalid mnemonic phrase");
    assert!(load_signer_from_env().is_err());

    // Test with an invalid hex string
    env::set_var(SIGNER_ENV, "0xinvalidhexstring");
    assert!(load_signer_from_env().is_err());

    // Test when the SIGNER environment variable is not set
    env::remove_var(SIGNER_ENV);
    assert!(load_signer_from_env().is_err());
    Ok(())
}

#[test]
fn test_load_evm_signer_from_env() -> color_eyre::Result<()> {
    color_eyre::install().unwrap_or(());
    let s = [1u8; 32];
    let secret = bip39::Mnemonic::from_entropy(&s[..])?.to_string();
    // Test with a valid mnemonic phrase
    env::set_var(EVM_SIGNER_ENV, secret);
    load_evm_signer_from_env()?;

    // Test with a valid hex string
    env::set_var(
        EVM_SIGNER_ENV,
        "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    load_evm_signer_from_env()?;

    // Test with an invalid mnemonic phrase
    env::set_var(EVM_SIGNER_ENV, "invalid mnemonic phrase");
    assert!(load_evm_signer_from_env().is_err());

    // Test with an invalid hex string
    env::set_var(EVM_SIGNER_ENV, "0xinvalidhexstring");
    assert!(load_evm_signer_from_env().is_err());

    // Test when the EVM_SIGNER environment variable is not set
    env::remove_var(EVM_SIGNER_ENV);
    assert!(load_evm_signer_from_env().is_err());

    Ok(())
}
