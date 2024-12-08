/// Trait for key types that can be stored in the keystore
pub trait KeyType: Sized + 'static {
    /// The public key type
    type Public: Clone;
    /// The secret key type
    type Secret: Clone;
    /// The signature type
    type Signature;

    /// Get a cryptographically secure random number generator
    fn get_rng() -> gadget_std::GadgetRng {
        gadget_std::GadgetRng::new()
    }

    /// Get a deterministic random number generator for testing
    fn get_test_rng() -> impl gadget_std::rand::Rng {
        gadget_std::test_rng()
    }

    /// Generate a new keypair with an optional seed
    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret, Error>;

    /// Get the public key from a secret key
    fn public_from_secret(secret: &Self::Secret) -> Self::Public;

    /// Sign a message with a secret key
    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature, Error>;

    /// Sign a pre-hashed message with a secret key
    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature, Error>;

    /// Verify a signature
    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool;
}

/// The keystore backend trait
pub trait Backend {
    /// Generate a new keypair
    fn generate_new<T: KeyType>(&self, seed: Option<&[u8]>) -> Result<T::Public, Error>;

    /// Sign a message with the key identified by the given public key
    fn sign<T: KeyType>(
        &self,
        public: &T::Public,
        msg: &[u8],
    ) -> Result<Option<T::Signature>, Error>;

    /// Get the secret key for a given public key
    fn expose_secret<T: KeyType>(&self, public: &T::Public) -> Result<Option<T::Secret>, Error>;

    /// Iterate over all public keys of a given type
    fn iter_keys<T: KeyType>(&self) -> Box<dyn Iterator<Item = T::Public>>;
}

/// Keystore errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Keystore lock error")]
    KeystoreLockError,
    #[error("Key type not supported")]
    KeyTypeNotSupported,
    #[error("Invalid message length")]
    InvalidMessageLength,
    #[error("Invalid seed")]
    InvalidSeed(String),
    #[error("Signature failed")]
    SignatureFailed(String),
    #[error("Error: {0}")]
    Other(String),
}
