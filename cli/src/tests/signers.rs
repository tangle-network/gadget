use crate::signer::{EVM_SIGNER_ENV, SIGNER_ENV, load_evm_signer_from_env, load_signer_from_env};
use color_eyre::eyre::Result;
use std::env;
use tangle_subxt::subxt_signer::bip39;

#[test]
fn test_load_signer_from_env() -> Result<()> {
    color_eyre::install().unwrap_or(());
    let s = [1u8; 32];
    let secret = bip39::Mnemonic::from_entropy(&s[..])?.to_string();
    // Test with a valid mnemonic phrase
    unsafe {
        env::set_var(SIGNER_ENV, secret);
    }
    load_signer_from_env()?;

    // Test with a valid hex string
    unsafe {
        env::set_var(
            SIGNER_ENV,
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
    }
    load_signer_from_env()?;

    // Test with an invalid mnemonic phrase
    unsafe {
        env::set_var(SIGNER_ENV, "invalid mnemonic phrase");
    }
    assert!(load_signer_from_env().is_err());

    // Test with an invalid hex string
    unsafe {
        env::set_var(SIGNER_ENV, "0xinvalidhexstring");
    }
    assert!(load_signer_from_env().is_err());

    // Test when the SIGNER environment variable is not set
    unsafe {
        env::remove_var(SIGNER_ENV);
    }
    assert!(load_signer_from_env().is_err());
    Ok(())
}

#[test]
fn test_load_evm_signer_from_env() -> Result<()> {
    color_eyre::install().unwrap_or(());
    let s = [1u8; 32];
    let secret = bip39::Mnemonic::from_entropy(&s[..])?.to_string();
    // Test with a valid mnemonic phrase
    unsafe {
        env::set_var(EVM_SIGNER_ENV, secret);
    }
    load_evm_signer_from_env()?;

    // Test with a valid hex string
    unsafe {
        env::set_var(
            EVM_SIGNER_ENV,
            "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
    }
    load_evm_signer_from_env()?;

    // Test with an invalid mnemonic phrase
    unsafe {
        env::set_var(EVM_SIGNER_ENV, "invalid mnemonic phrase");
    }
    assert!(load_evm_signer_from_env().is_err());

    // Test with an invalid hex string
    unsafe {
        env::set_var(EVM_SIGNER_ENV, "0xinvalidhexstring");
    }
    assert!(load_evm_signer_from_env().is_err());

    // Test when the EVM_SIGNER environment variable is not set
    unsafe {
        env::remove_var(EVM_SIGNER_ENV);
    }
    assert!(load_evm_signer_from_env().is_err());

    Ok(())
}
