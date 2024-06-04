//! Keystore Module for Gadget SDK
//!
//! Keystore is a module that provides an abstraction for a secure storage for private keys
//! and other sensitive data. It does this by relaying the storage to the underlying backend
//!
//! Currently, we support different backends:
//!
//! - `memory`: A simple in-memory storage (enabled by default)
//! - `file`: A storage that saves the keys to a file (`keystore-fs` feature)
//! - `kms`: A storage that uses a AWS Key Management System (`keystore-kms` feature)*
//! - `substrate`: Uses Substrate's Framework keystore (`keystore-substrate` feature)*
//! and a lot more to be added in the future.
//!
//! Note: The `kms` and `substrate` backends are not yet implemented.
//!
//! In Addition, the module provides a `Keystore` trait that can be implemented by any backend
//! and this trait offers the following methods:
//! - `sr25519` methods, to generate, import and export sr25519 keys
//! - `ed25519` methods, to generate, import and export ed25519 keys
//! - `ecdsa` methods, to generate, import and export ecdsa keys
//! - `bls381` methods, to generate, import and export bls381 keys
//!
//! with each module is feature gated to allow only the necessary dependencies to be included.

/// Backend modules
pub mod backend;
/// BLS381 Support
#[cfg(feature = "keystore-bls381")]
mod bls381;
/// ECDSA Support
#[cfg(feature = "keystore-ecdsa")]
mod ecdsa;
/// Ed25519 Support
#[cfg(feature = "keystore-ed25519")]
mod ed25519;
/// Keystore errors module
mod error;
/// Schnorrkel Support
#[cfg(feature = "keystore-sr25519")]
mod sr25519;

pub use error::Error;

/// The Keystore [`Backend`] trait defines the necessary functions that a keystore backend
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
    #[cfg(feature = "keystore-sr25519")]
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
    #[cfg(feature = "keystore-sr25519")]
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
    #[cfg(feature = "keystore-ed25519")]
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
    #[cfg(feature = "keystore-ed25519")]
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
    #[cfg(feature = "keystore-ecdsa")]
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
    #[cfg(feature = "keystore-ecdsa")]
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
    #[cfg(feature = "keystore-bls381")]
    fn bls381_generate_new(&self, seed: Option<&[u8]>) -> Result<bls381::Public, Error>;
    /// Generate an bls381 signature for a given message.
    ///
    /// Receives an [`bls381::Public`] key to be able to map
    /// it to a private key that exists in the keystore.
    ///
    /// Returns an [`bls381::Signature`] or `None` in case the given
    /// `public` doesn't exist in the keystore.
    /// # Errors
    /// An `Err` will be returned if generating the signature itself failed.
    #[cfg(feature = "keystore-bls381")]
    fn bls381_sign(
        &self,
        public: &bls381::Public,
        msg: &[u8],
    ) -> Result<Option<bls381::Signature>, Error>;
    /// Returns the Keypair for the given [`sr25519::Public`] if it does exist, otherwise returns `None`.
    /// # Errors
    /// An `Err` will be returned if finding the key operation itself failed.
    #[cfg(feature = "keystore-sr25519")]
    fn expose_sr25519_secret(
        &self,
        public: &sr25519::Public,
    ) -> Result<Option<sr25519::Secret>, Error>;
    /// Returns the [`ecdsa::Secret`] for the given [`ecdsa::Public`] if it does exist, otherwise returns `None`.
    /// # Errors
    /// An `Err` will be returned if finding the key operation itself failed.
    #[cfg(feature = "keystore-ecdsa")]
    fn expose_ecdsa_secret(&self, public: &ecdsa::Public) -> Result<Option<ecdsa::Secret>, Error>;
    /// Returns the [`ed25519::Secret`] for the given [`ed25519::Public`] if it does exist, otherwise returns `None`.
    /// # Errors
    /// An `Err` will be returned if finding the key operation itself failed.
    #[cfg(feature = "keystore-ed25519")]
    fn expose_ed25519_secret(
        &self,
        public: &ed25519::Public,
    ) -> Result<Option<ed25519::Secret>, Error>;
    /// Returns the [`bls381::Secret`] for the given [`bls381::Public`] if it does exist, otherwise returns `None`.
    /// # Errors
    /// An `Err` will be returned if finding the key operation itself failed.
    #[cfg(feature = "keystore-bls381")]
    fn expose_bls381_secret(
        &self,
        public: &bls381::Public,
    ) -> Result<Option<bls381::Secret>, Error>;
    /// Returns an iterator over all [`sr25519::Public`] keys that exist in the keystore.
    #[cfg(feature = "keystore-sr25519")]
    fn iter_sr25519(&self) -> impl Iterator<Item = sr25519::Public>;
    /// Returns an iterator over all [`bls381::Public`] keys that exist in the keystore.
    #[cfg(feature = "keystore-ecdsa")]
    fn iter_ecdsa(&self) -> impl Iterator<Item = ecdsa::Public>;
    /// Returns an iterator over all [`ed25519::Public`] keys that exist in the keystore.
    #[cfg(feature = "keystore-ed25519")]
    fn iter_ed25519(&self) -> impl Iterator<Item = ed25519::Public>;
    /// Returns an iterator over all [`bls381::Public`] keys that exist in the keystore.
    #[cfg(feature = "keystore-bls381")]
    fn iter_bls381(&self) -> impl Iterator<Item = bls381::Public>;
}
