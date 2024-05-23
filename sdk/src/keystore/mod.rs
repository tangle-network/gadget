//! Keystore Module for Gadget SDK
//!
//! Keystore is a module that provides an abstraction for a secure storage for private keys
//! and other sensitive data. It does this by relaying the storage to the underlying backend
//!
//! Currently, we support different backends:
//!
//! - `memory`: A simple in-memory storage (enabled by default)
//! - `file`: A storage that saves the keys to a file (`keystore-file` feature)
//! - `kms`: A storage that uses a AWS Key Management System (`keystore-kms` feature)
//! - `substrate`: Uses Substrate's Framewrok keystore (`keystore-substrate` feature)
//! and a lot more to be added in the future.
//!
//! In Addition, the module provides a `Keystore` trait that can be implemented by any backend
//! and this trait offers the following methods:
//! - `sr25519` methods, to generate, import and export sr25519 keys
//! - `ed25519` methods, to generate, import and export ed25519 keys
//! - `ecdsa` methods, to generate, import and export ecdsa keys
//! - `bls381` methods, to generate, import and export bls381 keys
//!
//! with each module is feature gated to allow only the necessary dependencies to be included.

/// Keystore errors module
mod error;
/// Backend modules
pub mod backend;

/// ECDSA Support
#[cfg(feature = "keystore-ecdsa")]
mod ecdsa;
/// Ed25519 Support
#[cfg(feature = "keystore-ed25519")]
mod ed25519;

pub use error::Error;

pub trait KeystoreBackend {
    /// Generate a new sr25519 key pair with an optional seed.
    ///
    /// Returns an `sr25519::Public` key of the generated key pair or an `Err` if
    /// something failed during key generation.
    #[cfg(feature = "keystore-sr25519")]
    fn sr25519_generate_new(&self, seed: Option<&[u8]>) -> Result<sr25519::Public, Error>;
    /// Generate an sr25519 signature for a given message.
    ///
    /// Receives an [`sr25519::Public`] key to be able to map
    /// them to a private key that exists in the keystore.
    ///
    /// Returns an [`sr25519::Signature`] or `None` in case the given
    /// `public` doesn't exist in the keystore.
    /// An `Err` will be returned if generating the signature itself failed.
    #[cfg(feature = "keystore-sr25519")]
    fn sr25519_sign(
        &self,
        public: &sr25519::Public,
        msg: &[u8],
    ) -> Result<Option<sr25519::Signature>, Error>;
    /// Generate a new ed25519 key pair with an optional seed.
    ///
    /// Returns an `ed25519::Public` key of the generated key pair or an `Err` if
    /// something failed during key generation.
    #[cfg(feature = "keystore-ed25519")]
    fn ed25519_generate_new(&self, seed: Option<&[u8]>) -> Result<ed25519::Public, Error>;
    /// Generate an ed25519 signature for a given message.
    ///
    /// Receives an [`ed25519::Public`] key to be able to map
    /// it to a private key that exists in the keystore.
    ///
    /// Returns an [`ed25519::Signature`] or `None` in case the given
    /// `public` doesn't exist in the keystore.
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
    #[cfg(feature = "keystore-ecdsa")]
    fn ecdsa_generate_new(&self, seed: Option<&[u8]>) -> Result<ecdsa::Public, Error>;
    /// Generate an ecdsa signature for a given message.
    ///
    /// Receives an [`ecdsa::Public`] key to be able to map
    /// it to a private key that exists in the keystore.
    ///
    /// Returns an [`ecdsa::Signature`] or `None` in case the given
    /// `public` doesn't exist in the keystore.
    /// An `Err` will be returned if generating the signature itself failed.
    #[cfg(feature = "keystore-ecdsa")]
    fn ecdsa_sign(
        &self,
        public: &ecdsa::Public,
        msg: &[u8],
    ) -> Result<Option<ecdsa::Signature>, Error>;
    /// Generate a new bls381 key pair with an optional seed.
    ///
    /// Returns an `bls381::Public` key of the generated key pair or an `Err` if
    /// something failed during key generation.
    #[cfg(feature = "keystore-bls381")]
    fn bls381_generate_new(&self, seed: Option<&[u8]>) -> Result<bls381::Public, Error>;
    /// Generate an bls381 signature for a given message.
    ///
    /// Receives an [`bls381::Public`] key to be able to map
    /// it to a private key that exists in the keystore.
    ///
    /// Returns an [`bls381::Signature`] or `None` in case the given
    /// `public` doesn't exist in the keystore.
    /// An `Err` will be returned if generating the signature itself failed.
    #[cfg(feature = "keystore-bls381")]
    fn bls381_sign(
        &self,
        public: &bls381::Public,
        msg: &[u8],
    ) -> Result<Option<bls381::Signature>, Error>;
    /// Checks if the private key for the given public key exist.
    ///
    /// Returns `true` if the private key could be found.
    fn has_key(&self, public: &[u8]) -> bool;
    /// Returns the Keypair for the given [`sr25519::Public`] if it does exist, otherwise returns `None`.
    /// An `Err` will be returned if finding the key operation itself failed.
    #[cfg(feature = "keystore-sr25519")]
    fn expose_sr25519_secret(&self, public: &sr25519::Public)
        -> Result<Option<sr25519::Secret>, Error>;

    /// Returns the [`ecdsa::Secret`] for the given [`ecdsa::Public`] if it does exist, otherwise returns `None`.
    /// An `Err` will be returned if finding the key operation itself failed.
    #[cfg(feature = "keystore-ecdsa")]
    fn expose_ecdsa_secret(&self, public: &ecdsa::Public) -> Result<Option<ecdsa::Secret>, Error>;
    /// Returns the [`ed25519::Secret`] for the given [`ed25519::Public`] if it does exist, otherwise returns `None`.
    /// An `Err` will be returned if finding the key operation itself failed.
    #[cfg(feature = "keystore-ed25519")]
    fn expose_ed25519_secret(&self, public: &ed25519::Public)
        -> Result<Option<ed25519::Secret>, Error>;
    /// Returns the [`bls381::Secret`] for the given [`bls381::Public`] if it does exist, otherwise returns `None`.
    /// An `Err` will be returned if finding the key operation itself failed.
    #[cfg(feature = "keystore-bls381")]
    fn expose_bls381_secret(&self, public: &bls381::Public) -> Result<Option<bls381::Secret>, Error>;
}
