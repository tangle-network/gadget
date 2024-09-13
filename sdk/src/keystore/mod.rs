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

use eigensdk_rs::eigen_utils::crypto::bls::{self as bls_bn254, g1_point_to_g1_projective};
pub use error::Error;
use subxt::{PolkadotConfig, SubstrateConfig};
use tangle_subxt::subxt::ext::sp_core;
use tangle_subxt::subxt::tx::PairSigner;
pub type TanglePairSigner = PairSigner<SubstrateConfig, sp_core::sr25519::Pair>;
pub type TanglePairSignerPolkadot = PairSigner<PolkadotConfig, sp_core::sr25519::Pair>;

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

pub trait BackendExt: Backend {
    fn ecdsa_key(&self) -> Result<subxt_signer::ecdsa::Keypair, Error> {
        let first_key = self
            .iter_ecdsa()
            .next()
            .ok_or_else(|| str_to_std_error("No ECDSA keys found"))?;
        let ecdsa_secret = self
            .expose_ecdsa_secret(&first_key)?
            .ok_or_else(|| str_to_std_error("No ECDSA secret found"))?;

        let mut seed = [0u8; 32];
        seed.copy_from_slice(&ecdsa_secret.to_bytes()[0..32]);
        Ok(
            tangle_subxt::subxt_signer::ecdsa::Keypair::from_secret_key(seed)
                .map_err(|err| str_to_std_error(err.to_string()))?,
        )
    }

    fn sr25519_key(&self) -> Result<TanglePairSigner, Error> {
        let first_key = self
            .iter_sr25519()
            .next()
            .ok_or_else(|| str_to_std_error("No SR25519 keys found"))?;
        let secret = self
            .expose_sr25519_secret(&first_key)?
            .ok_or_else(|| str_to_std_error("No SR25519 secret found"))?;
        //et pair = gadget_common::tangle_subxt::subxt::ext::sp_core::sr25519::Pair::from(secret);
        let schnorrkel_kp = schnorrkel::Keypair::from(secret);
        let res = PairSigner::new(schnorrkel_kp.into());
        Ok(res)
    }

    fn sr25519_key_polkadot(&self) -> Result<TanglePairSignerPolkadot, Error> {
        let first_key = self
            .iter_sr25519()
            .next()
            .ok_or_else(|| str_to_std_error("No SR25519 keys found"))?;
        let secret = self
            .expose_sr25519_secret(&first_key)?
            .ok_or_else(|| str_to_std_error("No SR25519 secret found"))?;
        //et pair = gadget_common::tangle_subxt::subxt::ext::sp_core::sr25519::Pair::from(secret);
        let schnorrkel_kp = schnorrkel::Keypair::from(secret);
        let res = PairSigner::new(schnorrkel_kp.into());
        Ok(res)
    }

    fn bls_bn254_key(&self) -> Result<bls_bn254::KeyPair, Error> {
        let first_key = self
            .iter_bls_bn254()
            .next()
            .ok_or_else(|| str_to_std_error("No BLS BN254 keys found"))?;
        let bls_secret = self
            .expose_bls_bn254_secret(&first_key)?
            .ok_or_else(|| str_to_std_error("No BLS BN254 secret found"))?;

        Ok(bls_bn254::KeyPair {
            priv_key: bls_secret,
            pub_key: g1_point_to_g1_projective(&first_key),
        })
    }
}

impl<T: Backend> BackendExt for T {}

fn str_to_std_error<T: Into<String>>(s: T) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, s.into())
}
