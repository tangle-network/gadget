//! This module contains the key types that are supported by the keystore.

use crate::error::Result;
use serde::Deserialize;
use serde::Serialize;

#[cfg(feature = "bn254")]
use ark_serialize::CanonicalDeserialize;
#[cfg(feature = "bn254")]
use ark_serialize::CanonicalSerialize;

/// Tangle specific key types
#[cfg(feature = "tangle")]
pub mod sp_core_util;

/// Key types that are used separately from Tangle defaults
#[cfg(feature = "bn254")]
pub mod arkworks_bn254;
#[cfg(feature = "ecdsa")]
pub mod k256_ecdsa;
#[cfg(feature = "sr25519-schnorrkel")]
pub mod schnorrkel_sr25519;
#[cfg(feature = "bls381")]
pub mod w3f_bls381;
#[cfg(feature = "zebra")]
pub mod zebra_ed25519;

// Re-export the types
#[cfg(feature = "tangle")]
pub use self::sp_core_util::*;

/// Serialize this to a vector of bytes.
#[cfg(feature = "bn254")]
pub fn to_bytes<T: CanonicalSerialize>(elt: T) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(elt.compressed_size());

    <T as CanonicalSerialize>::serialize_compressed(&elt, &mut bytes).unwrap();

    bytes
}

/// Deserialize this from a slice of bytes.
#[cfg(feature = "bn254")]
pub fn from_bytes<T: CanonicalDeserialize>(bytes: &[u8]) -> T {
    <T as CanonicalDeserialize>::deserialize_compressed(&mut &bytes[..]).unwrap()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum KeyTypeId {
    #[cfg(feature = "bn254")]
    ArkBn254,
    #[cfg(feature = "ecdsa")]
    K256Ecdsa,
    #[cfg(feature = "sr25519-schnorrkel")]
    SchnorrkelSr25519,
    #[cfg(feature = "bls381")]
    W3fBls381,
    #[cfg(feature = "zebra")]
    ZebraEd25519,
    #[cfg(feature = "tangle")]
    SpBls377,
    #[cfg(feature = "tangle")]
    SpBls381,
    #[cfg(feature = "tangle")]
    SpEcdsa,
    #[cfg(feature = "tangle")]
    SpEd25519,
    #[cfg(feature = "tangle")]
    SpSr25519,
}

impl KeyTypeId {
    pub fn name(&self) -> &'static str {
        match self {
            #[cfg(feature = "bn254")]
            KeyTypeId::ArkBn254 => "ark_bn254",
            #[cfg(feature = "ecdsa")]
            KeyTypeId::K256Ecdsa => "k256_ecdsa",
            #[cfg(feature = "sr25519-schnorrkel")]
            KeyTypeId::SchnorrkelSr25519 => "schnorrkel_sr25519",
            #[cfg(feature = "bls381")]
            KeyTypeId::W3fBls381 => "w3f_bls381",
            #[cfg(feature = "zebra")]
            KeyTypeId::ZebraEd25519 => "zebra_ed25519",
            #[cfg(feature = "tangle")]
            KeyTypeId::SpBls377 => "sp_bls377",
            #[cfg(feature = "tangle")]
            KeyTypeId::SpBls381 => "sp_bls381",
            #[cfg(feature = "tangle")]
            KeyTypeId::SpEcdsa => "sp_ecdsa",
            #[cfg(feature = "tangle")]
            KeyTypeId::SpEd25519 => "sp_ed25519",
            #[cfg(feature = "tangle")]
            KeyTypeId::SpSr25519 => "sp_sr25519",
        }
    }
}

/// Trait for key types that can be stored in the keystore
pub trait KeyType: Sized + 'static {
    /// The public key type
    type Public: Clone + Serialize + serde::de::DeserializeOwned + Ord + Send + Sync;
    /// The secret key type
    type Secret: Clone + Serialize + serde::de::DeserializeOwned + Ord + Send + Sync;
    /// The signature type
    type Signature: Clone + Serialize + serde::de::DeserializeOwned + Ord + Send + Sync;

    /// Get the key type identifier
    fn key_type_id() -> KeyTypeId;

    /// Get a cryptographically secure random number generator
    fn get_rng() -> gadget_std::GadgetRng {
        gadget_std::GadgetRng::new()
    }

    /// Get a deterministic random number generator for testing
    fn get_test_rng() -> impl gadget_std::CryptoRng + gadget_std::Rng {
        gadget_std::test_rng()
    }

    /// Generate a new keypair with an optional seed
    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret>;

    /// Generate a new keypair with the secret string
    fn generate_with_string(secret: String) -> Result<Self::Secret>;

    /// Get the public key from a secret key
    fn public_from_secret(secret: &Self::Secret) -> Self::Public;

    /// Sign a message with a secret key
    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature>;

    /// Sign a pre-hashed message with a secret key
    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature>;

    /// Verify a signature
    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool;
}
