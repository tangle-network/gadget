//! Keystore Module for Gadget SDK
//!
//! Keystore is a module that provides an abstraction for a secure storage for private keys
//! and other sensitive data. It does this by relaying the storage to the underlying backend
//!
//! Currently, we support different backends:
//!
//! - `memory`: A simple in-memory storage (enabled by default)
//! - `file`: A storage that saves the keys to a file (`std` feature, enabled by default)
//! - `kms`: A storage that uses an AWS Key Management System (`keystore-kms` feature)*
//! - `substrate`: Uses Substrate's Framework keystore (`keystore-substrate` feature)*
//! and a lot more to be added in the future.
//!
//! Note: The `kms` and `substrate` backends are not yet implemented.
//!
//! In Addition, the module provides a `Keystore` trait that can be implemented by any backend
//! and this trait offers the following methods:
//! - `sr25519` methods, to generate, import, and export sr25519 keys
//! - `ed25519` methods, to generate, import, and export ed25519 keys
//! - `ecdsa` methods, to generate, import, and export ecdsa keys
//! - `bls381` methods, to generate, import, and export bls381 keys
//! - `bn254` methods, to generate, import, and export bn254 bls keys
//!
//! with each module is feature gated to allow only the necessary dependencies to be included.

/// Backend modules
pub mod backend;

/// BLS381 Support
pub mod bls381;

/// BLS BN254 Support
pub mod bn254;

/// ECDSA Support
pub mod ecdsa;

/// Ed25519 Support
pub mod ed25519;

/// Keystore errors module
pub mod error;

/// Schnorrkel Support
pub mod sr25519;
#[cfg(test)]
mod tests;

use crate::clients::tangle::runtime::TangleConfig;
#[cfg(any(feature = "std", feature = "wasm"))]
use alloy_signer_local::LocalSigner;
use eigensdk::crypto_bls;
pub use error::Error;
use k256::ecdsa::SigningKey;
use sp_core::crypto::{DeriveError, SecretStringError};
use sp_core::DeriveJunction;
use std::path::{Path, PathBuf};
use subxt::ext::sp_core::Pair as PairSubxt;
use subxt_core::tx::signer::{PairSigner, Signer};
use subxt_core::utils::{AccountId32, MultiAddress, MultiSignature};
#[cfg(any(feature = "std", feature = "wasm"))]
use tangle_subxt::subxt;

#[cfg(any(feature = "std", feature = "wasm"))]
#[derive(Clone, Debug)]
pub struct TanglePairSigner<Pair> {
    pub(crate) pair: subxt::tx::PairSigner<TangleConfig, Pair>,
}

#[cfg(any(feature = "std", feature = "wasm"))]
impl<Pair: sp_core::Pair> sp_core::crypto::CryptoType for TanglePairSigner<Pair> {
    type Pair = Pair;
}

#[cfg(any(feature = "std", feature = "wasm"))]
impl<Pair: sp_core::Pair> TanglePairSigner<Pair>
where
    <Pair as sp_core::Pair>::Signature: Into<MultiSignature>,
    subxt::ext::sp_runtime::MultiSigner: From<<Pair as sp_core::Pair>::Public>,
{
    pub fn new(pair: Pair) -> Self {
        TanglePairSigner {
            pair: PairSigner::new(pair),
        }
    }

    pub fn into_inner(self) -> PairSigner<TangleConfig, Pair> {
        self.pair
    }

    pub fn signer(&self) -> &Pair {
        self.pair.signer()
    }
}

#[cfg(any(feature = "std", feature = "wasm"))]
impl<Pair> Signer<TangleConfig> for TanglePairSigner<Pair>
where
    Pair: sp_core::Pair,
    Pair::Signature: Into<MultiSignature>,
{
    fn account_id(&self) -> AccountId32 {
        self.pair.account_id()
    }

    fn address(&self) -> MultiAddress<AccountId32, ()> {
        self.pair.address()
    }

    fn sign(&self, signer_payload: &[u8]) -> MultiSignature {
        self.pair.sign(signer_payload)
    }
}

#[cfg(any(feature = "std", feature = "wasm"))]
impl<Pair: sp_core::Pair> sp_core::Pair for TanglePairSigner<Pair>
where
    <Pair as sp_core::Pair>::Signature: Into<subxt::utils::MultiSignature>,
    subxt::ext::sp_runtime::MultiSigner: From<<Pair as sp_core::Pair>::Public>,
{
    type Public = Pair::Public;
    type Seed = Pair::Seed;
    type Signature = Pair::Signature;

    fn derive<Iter: Iterator<Item = DeriveJunction>>(
        &self,
        path: Iter,
        seed: Option<Self::Seed>,
    ) -> Result<(Self, Option<Self::Seed>), DeriveError> {
        Pair::derive(self.pair.signer(), path, seed).map(|(pair, seed)| {
            (
                TanglePairSigner {
                    pair: PairSigner::new(pair),
                },
                seed,
            )
        })
    }

    fn from_seed_slice(seed: &[u8]) -> Result<Self, SecretStringError> {
        Pair::from_seed_slice(seed).map(|pair| TanglePairSigner {
            pair: PairSigner::new(pair),
        })
    }

    fn sign(&self, message: &[u8]) -> Self::Signature {
        Pair::sign(self.pair.signer(), message)
    }

    fn verify<M: AsRef<[u8]>>(sig: &Self::Signature, message: M, pubkey: &Self::Public) -> bool {
        Pair::verify(sig, message, pubkey)
    }

    fn public(&self) -> Self::Public {
        Pair::public(self.pair.signer())
    }

    fn to_raw_vec(&self) -> Vec<u8> {
        Pair::to_raw_vec(self.pair.signer())
    }
}

impl TanglePairSigner<sp_core::ecdsa::Pair> {
    /// Returns the alloy-compatible key for the ECDSA key pair.
    pub fn alloy_key(&self) -> Result<LocalSigner<SigningKey>, Error> {
        let k256_ecdsa_secret_key = self.pair.signer().seed();
        let res = LocalSigner::from_slice(&k256_ecdsa_secret_key)
            .map_err(|err| Error::Alloy(err.to_string()))?;
        Ok(res)
    }
}

/// The Keystore [`Backend`] trait.
///
/// It defines the necessary functions that a keystore backend
/// must implement to support various cryptographic key operations such as key generation,
/// signing, and public key retrieval.
pub trait Backend {
    /// Generate a new sr25519 key pair with an optional seed.
    ///
    /// Returns an [`sr25519::Public`] key of the generated key pair or an `Err` if
    /// something failed during key generation.
    ///
    /// # Errors
    /// An `Err` will be returned if generating the key pair operation itself failed.
    fn sr25519_generate_new(&self, seed: Option<&[u8]>) -> Result<sr25519::Public, Error>;
    /// Generate an sr25519 signature for a given message.
    ///
    /// Receives an [`sr25519::Public`] key to be able to map
    /// them to a private key that exists in the keystore.
    ///
    /// Returns an [`sr25519::Signature`] or `None` in case the given
    /// `public` doesn't exist in the keystore.
    /// # Errors
    /// An `Err` will be returned if generating the signature itself failed.
    fn sr25519_sign(
        &self,
        public: &sr25519::Public,
        msg: &[u8],
    ) -> Result<Option<sr25519::Signature>, Error>;
    /// Generate a new ed25519 key pair with an optional seed.
    ///
    /// Returns an [`ed25519::Public`] key of the generated key pair or an `Err` if
    /// something failed during key generation.
    ///
    /// # Errors
    /// An `Err` will be returned if generating the key pair operation itself failed.
    fn ed25519_generate_new(&self, seed: Option<&[u8]>) -> Result<ed25519::Public, Error>;
    /// Generate an ed25519 signature for a given message.
    ///
    /// Receives an [`ed25519::Public`] key to be able to map
    /// it to a private key that exists in the keystore.
    ///
    /// Returns an [`ed25519::Signature`] or `None` in case the given
    /// `public` doesn't exist in the keystore.
    /// # Errors
    /// An `Err` will be returned if generating the signature itself failed.
    fn ed25519_sign(
        &self,
        public: &ed25519::Public,
        msg: &[u8],
    ) -> Result<Option<ed25519::Signature>, Error>;
    /// Generate a new ecdsa key pair with an optional seed.
    ///
    /// Returns an `ecdsa::Public` key of the generated key pair or an `Err` if
    /// something failed during key generation.
    /// # Errors
    /// An `Err` will be returned if generating the key pair operation itself failed.
    fn ecdsa_generate_new(&self, seed: Option<&[u8]>) -> Result<ecdsa::Public, Error>;
    /// Generate an ecdsa signature for a given message.
    ///
    /// Receives an [`ecdsa::Public`] key to be able to map
    /// it to a private key that exists in the keystore.
    ///
    /// Returns an [`ecdsa::Signature`] or `None` in case the given
    /// `public` doesn't exist in the keystore.
    /// # Errors
    /// An `Err` will be returned if generating the signature itself failed.
    fn ecdsa_sign(
        &self,
        public: &ecdsa::Public,
        msg: &[u8],
    ) -> Result<Option<ecdsa::Signature>, Error>;
    /// Generate a new bls381 key pair with an optional seed.
    ///
    /// Returns an [`bls381::Public`] key of the generated key pair or an `Err` if
    /// something failed during key generation.
    /// # Errors
    /// An `Err` will be returned if generating the key pair operation itself failed.
    fn bls381_generate_new(&self, seed: Option<&[u8]>) -> Result<bls381::Public, Error>;
    /// Generate a bls381 signature for a given message.
    ///
    /// Receives an [`bls381::Public`] key to be able to map
    /// it to a private key that exists in the keystore.
    ///
    /// Returns an [`bls381::Signature`] or `None` in case the given
    /// `public` doesn't exist in the keystore.
    /// # Errors
    /// An `Err` will be returned if generating the signature itself failed.
    fn bls381_sign(
        &self,
        public: &bls381::Public,
        msg: &[u8],
    ) -> Result<Option<bls381::Signature>, Error>;
    /// Generate a new bls bn254 key pair with an optional seed.
    ///
    /// Returns an [`bn254::Public`] key of the generated key pair or an `Err` if
    /// something failed during key generation.
    /// # Errors
    /// An `Err` will be returned if generating the key pair operation itself failed.
    fn bls_bn254_generate_new(&self, seed: Option<&[u8]>) -> Result<bn254::Public, Error>;
    /// Generate a new bls bn254 key pair from a [`bn254::Secret`] hex String
    ///
    /// Returns an [`bn254::Public`] key of the generated key pair or an `Err` if
    /// something failed during key generation.
    /// # Errors
    /// An `Err` will be returned if generating the key pair operation itself failed.
    fn bls_bn254_generate_from_secret(&self, secret: String) -> Result<bn254::Public, Error>;
    /// Generate a bls bn254 signature for a given message.
    ///
    /// Receives an [`bn254::Public`] key to be able to map
    /// it to a private key that exists in the keystore.
    ///
    /// Returns an [`bn254::Signature`] or `None` in case the given
    /// `public` doesn't exist in the keystore.
    /// # Errors
    /// An `Err` will be returned if generating the signature itself failed.
    fn bls_bn254_sign(
        &self,
        public: &bn254::Public,
        msg: &[u8; 32],
    ) -> Result<Option<bn254::Signature>, Error>;
    /// Returns the Keypair for the given [`sr25519::Public`] if it does exist, otherwise returns `None`.
    /// # Errors
    /// An `Err` will be returned if finding the key operation itself failed.
    fn expose_sr25519_secret(
        &self,
        public: &sr25519::Public,
    ) -> Result<Option<sr25519::Secret>, Error>;
    /// Returns the [`ecdsa::Secret`] for the given [`ecdsa::Public`] if it does exist, otherwise returns `None`.
    /// # Errors
    /// An `Err` will be returned if finding the key operation itself failed.
    fn expose_ecdsa_secret(&self, public: &ecdsa::Public) -> Result<Option<ecdsa::Secret>, Error>;
    /// Returns the [`ed25519::Secret`] for the given [`ed25519::Public`] if it does exist, otherwise returns `None`.
    /// # Errors
    /// An `Err` will be returned if finding the key operation itself failed.
    fn expose_ed25519_secret(
        &self,
        public: &ed25519::Public,
    ) -> Result<Option<ed25519::Secret>, Error>;
    /// Returns the [`bls381::Secret`] for the given [`bls381::Public`] if it does exist, otherwise returns `None`.
    /// # Errors
    /// An `Err` will be returned if finding the key operation itself failed.
    fn expose_bls381_secret(
        &self,
        public: &bls381::Public,
    ) -> Result<Option<bls381::Secret>, Error>;
    /// Returns the [`bn254::Secret`] for the given [`bn254::Public`] if it does exist, otherwise returns `None`.
    /// # Errors
    /// An `Err` will be returned if finding the key operation itself failed.
    fn expose_bls_bn254_secret(
        &self,
        public: &bn254::Public,
    ) -> Result<Option<bn254::Secret>, Error>;
    /// Returns an iterator over all [`sr25519::Public`] keys that exist in the keystore.
    fn iter_sr25519(&self) -> impl Iterator<Item = sr25519::Public>;
    /// Returns an iterator over all [`ecdsa::Public`] keys that exist in the keystore.
    fn iter_ecdsa(&self) -> impl Iterator<Item = ecdsa::Public>;
    /// Returns an iterator over all [`ed25519::Public`] keys that exist in the keystore.
    fn iter_ed25519(&self) -> impl Iterator<Item = ed25519::Public>;
    /// Returns an iterator over all [`bls381::Public`] keys that exist in the keystore.
    fn iter_bls381(&self) -> impl Iterator<Item = bls381::Public>;
    /// Returns an iterator over all [`bn254::Public`] keys that exist in the keystore.
    fn iter_bls_bn254(&self) -> impl Iterator<Item = bn254::Public>;
}

/// A convenience trait to extend the [`Backend`] trait with additional methods
/// that provide convenient access to keys
pub trait BackendExt: Backend {
    #[cfg(any(feature = "std", feature = "wasm"))]
    fn ecdsa_key(&self) -> Result<TanglePairSigner<sp_core::ecdsa::Pair>, Error> {
        let first_key = self
            .iter_ecdsa()
            .next()
            .ok_or_else(|| Error::Ecdsa("No ECDSA Public Keys".to_string()))?;
        let ecdsa_secret = self
            .expose_ecdsa_secret(&first_key)?
            .ok_or_else(|| Error::Ecdsa("No corresponding ECDSA secret key found".to_string()))?;

        let mut seed = [0u8; 32];
        seed.copy_from_slice(&ecdsa_secret.to_bytes()[0..32]);
        Ok(TanglePairSigner {
            pair: subxt::tx::PairSigner::new(sp_core::ecdsa::Pair::from_seed(&seed)),
        })
    }

    #[cfg(any(feature = "std", feature = "wasm"))]
    fn sr25519_key(&self) -> Result<TanglePairSigner<sp_core::sr25519::Pair>, Error> {
        let first_key = self
            .iter_sr25519()
            .next()
            .ok_or_else(|| Error::Sr25519("No Sr25519 public key found".to_string()))?;
        let secret = self
            .expose_sr25519_secret(&first_key)?
            .ok_or_else(|| Error::Sr25519("No SR25519 secret found".to_string()))?;
        let schnorrkel_kp = schnorrkel::Keypair::from(secret);
        let pair = subxt::tx::PairSigner::new(schnorrkel_kp.into());
        Ok(TanglePairSigner { pair })
    }

    fn ed25519_key(&self) -> Result<TanglePairSigner<sp_core::ed25519::Pair>, Error> {
        let first_key = self
            .iter_ed25519()
            .next()
            .ok_or_else(|| Error::Ed25519("No ED25519 keys found".to_string()))?;
        let ed25519_secret = self
            .expose_ed25519_secret(&first_key)?
            .ok_or_else(|| Error::Ed25519("No ED25519 secret found".to_string()))?;

        let mut seed = [0u8; 32];
        seed.copy_from_slice(&ed25519_secret.as_ref()[0..32]);
        Ok(TanglePairSigner {
            pair: subxt::tx::PairSigner::new(sp_core::ed25519::Pair::from_seed(&seed)),
        })
    }

    #[cfg(any(feature = "std", feature = "wasm"))]
    fn bls_bn254_key(&self) -> Result<crypto_bls::BlsKeyPair, Error> {
        let first_key = self
            .iter_bls_bn254()
            .next()
            .ok_or_else(|| Error::BlsBn254("No BLS BN254 keys found".to_string()))?;
        let bls_secret = self
            .expose_bls_bn254_secret(&first_key)?
            .ok_or_else(|| Error::BlsBn254("No BLS BN254 secret found".to_string()))?;
        crypto_bls::BlsKeyPair::new(bls_secret.to_string())
            .map_err(|e| Error::BlsBn254(e.to_string()))
    }
}

impl<T: Backend> BackendExt for T {}

pub trait KeystoreUriSanitizer: AsRef<Path> {
    fn sanitize_file_path(&self) -> PathBuf {
        let this: PathBuf = self.as_ref().to_path_buf();
        let mut path = format!("{:?}", this.display());
        path = path.replace("file:///", "/");
        path = path.replace("file://", "/");
        path = path.replace("file:/", "/");
        path = path.replace("file:", "");

        path = path.replace("\"", "");

        path.into()
    }
}

impl<T: AsRef<Path>> KeystoreUriSanitizer for T {}
