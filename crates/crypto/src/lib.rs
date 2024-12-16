#![cfg_attr(not(feature = "std"), no_std)]

use thiserror::Error;

pub use gadget_crypto_core::*;

#[cfg(feature = "k256")]
pub use gadget_crypto_k256 as k256_crypto;

#[cfg(feature = "sr25519-schnorrkel")]
pub use gadget_crypto_sr25519 as sr25519_crypto;

#[cfg(feature = "ed25519")]
pub use gadget_crypto_ed25519 as ed25519_crypto;

#[cfg(feature = "bls")]
pub use gadget_crypto_bls as bls_crypto;

#[cfg(feature = "bn254")]
pub use gadget_crypto_bn254 as bn254_crypto;

#[cfg(feature = "sp-core")]
pub use gadget_crypto_sp_core as sp_core_crypto;

#[derive(Debug, Error)]
pub enum CryptoCoreError {
    #[cfg(feature = "k256")]
    #[error(transparent)]
    K256(#[from] gadget_crypto_k256::error::K256Error),
    #[cfg(feature = "sr25519-schnorrkel")]
    #[error(transparent)]
    Sr25519(#[from] gadget_crypto_sr25519::error::Sr25519Error),
    #[cfg(feature = "ed25519")]
    #[error(transparent)]
    Ed25519(#[from] gadget_crypto_ed25519::error::Ed25519Error),
    #[cfg(feature = "bls")]
    #[error(transparent)]
    Bls(#[from] gadget_crypto_bls::error::BlsError),
    #[cfg(feature = "bn254")]
    #[error(transparent)]
    Bn254(#[from] gadget_crypto_bn254::error::Bn254Error),
    #[cfg(feature = "sp-core")]
    #[error(transparent)]
    SpCore(#[from] gadget_crypto_sp_core::error::SpCoreError),
}

pub trait IntoCryptoError {
    fn into_crypto_error(self) -> CryptoCoreError;
}

#[cfg(feature = "k256")]
impl IntoCryptoError for gadget_crypto_k256::error::K256Error {
    fn into_crypto_error(self) -> CryptoCoreError {
        CryptoCoreError::K256(self)
    }
}

#[cfg(feature = "sr25519-schnorrkel")]
impl IntoCryptoError for gadget_crypto_sr25519::error::Sr25519Error {
    fn into_crypto_error(self) -> CryptoCoreError {
        CryptoCoreError::Sr25519(self)
    }
}

#[cfg(feature = "ed25519")]
impl IntoCryptoError for gadget_crypto_ed25519::error::Ed25519Error {
    fn into_crypto_error(self) -> CryptoCoreError {
        CryptoCoreError::Ed25519(self)
    }
}

#[cfg(feature = "bls")]
impl IntoCryptoError for gadget_crypto_bls::error::BlsError {
    fn into_crypto_error(self) -> CryptoCoreError {
        CryptoCoreError::Bls(self)
    }
}

#[cfg(feature = "bn254")]
impl IntoCryptoError for gadget_crypto_bn254::error::Bn254Error {
    fn into_crypto_error(self) -> CryptoCoreError {
        CryptoCoreError::Bn254(self)
    }
}

#[cfg(feature = "sp-core")]
impl IntoCryptoError for gadget_crypto_sp_core::error::SpCoreError {
    fn into_crypto_error(self) -> CryptoCoreError {
        CryptoCoreError::SpCore(self)
    }
}
