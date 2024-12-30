use crate::signer::{load_evm_signer_from_env, load_signer_from_env, EVM_SIGNER_ENV, SIGNER_ENV};
use color_eyre::eyre::Result;
use gadget_crypto::bn254_crypto::ArkBlsBn254;
use gadget_crypto_core::KeyTypeId;
use gadget_keystore::backends::tangle::TangleBackend;
use gadget_keystore::backends::tangle_bls::TangleBlsBackend;
use gadget_keystore::backends::Backend;
use gadget_keystore::{Keystore, KeystoreConfig};
use std::env;
use std::process::Command;
use tangle_subxt::subxt_signer::bip39;
use tempfile::tempdir;

#[test]
fn test_cli_fs_key_generation() -> Result<()> {
    let temp_dir = tempdir()?;
    let output_path = temp_dir.path();
    let keystore = Keystore::new(KeystoreConfig::new().fs_root(output_path))?;

    for key_type in [
        KeyTypeId::SpSr25519,
        KeyTypeId::SpEd25519,
        KeyTypeId::SpEcdsa,
        KeyTypeId::SpBls381,
        KeyTypeId::ArkBn254,
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
            .arg(output_path)
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
            KeyTypeId::SpSr25519 => {
                let public = keystore.iter_sr25519().next().unwrap();
                assert!(!public.is_empty());
            }
            KeyTypeId::SpEd25519 => {
                let public = keystore.iter_ed25519().next().unwrap();
                assert!(!public.is_empty());
            }
            KeyTypeId::SpEcdsa => {
                let public = keystore.iter_ecdsa().next().unwrap();
                assert!(!public.0.is_empty());
            }
            KeyTypeId::SpBls381 => {
                let public = keystore.iter_bls381().next().unwrap();
                assert!(!public.is_empty());
            }
            KeyTypeId::ArkBn254 => {
                let keys = keystore.list_local::<ArkBlsBn254>().unwrap();
                assert!(keys.first().is_some());
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}

#[test]
fn test_cli_mem_key_generation() -> Result<()> {
    for key_type in [
        KeyTypeId::SpSr25519,
        KeyTypeId::SpEd25519,
        KeyTypeId::SpEcdsa,
        KeyTypeId::SpBls381,
        KeyTypeId::ArkBn254,
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
