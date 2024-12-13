//! This module contains the key types that are supported by the keystore.

use crate::error::Result;
use gadget_std::string::String;
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
#[cfg(any(feature = "bls377", feature = "bls381"))]
pub mod w3f_bls;
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
    #[cfg(any(feature = "bls377", feature = "bls381"))]
    W3fBls,
    #[cfg(feature = "zebra")]
    ZebraEd25519,
    #[cfg(feature = "tangle")]
    TangleSr25519,
}

impl KeyTypeId {
    pub const ENABLED: &'static [Self] = &[
        #[cfg(feature = "bn254")]
        Self::ArkBn254,
        #[cfg(feature = "ecdsa")]
        Self::K256Ecdsa,
        #[cfg(feature = "sr25519-schnorrkel")]
        Self::SchnorrkelSr25519,
        #[cfg(feature = "bls377")]
        Self::W3fBls377,
        #[cfg(feature = "bls381")]
        Self::W3fBls381,
        #[cfg(feature = "zebra")]
        Self::ZebraEd25519,
        #[cfg(feature = "tangle")]
        Self::SpBls377,
        #[cfg(feature = "tangle")]
        Self::SpBls381,
        #[cfg(feature = "tangle")]
        Self::SpEcdsa,
        #[cfg(feature = "tangle")]
        Self::SpEd25519,
        #[cfg(feature = "tangle")]
        Self::SpSr25519,
    ];

    pub fn name(&self) -> &'static str {
        match *self {
            #[cfg(feature = "bn254")]
            Self::ArkBn254 => "ark-bn254",
            #[cfg(feature = "ecdsa")]
            Self::K256Ecdsa => "k256-ecdsa",
            #[cfg(feature = "sr25519-schnorrkel")]
            Self::SchnorrkelSr25519 => "schnorrkel-sr25519",
            #[cfg(any(feature = "bls377", feature = "bls381"))]
            Self::W3fBls => "w3f-bls",
            #[cfg(feature = "zebra")]
            Self::ZebraEd25519 => "zebra-ed25519",
            #[cfg(feature = "tangle")]
            Self::TangleSr25519 => "tangle-sr25519",
            #[cfg(all(
                not(feature = "bn254"),
                not(feature = "ecdsa"),
                not(feature = "sr25519-schnorrkel"),
                not(any(feature = "bls377", feature = "bls381")),
                not(feature = "zebra"),
                not(feature = "tangle")
            ))]
            _ => unreachable!("All possible variants are feature-gated"),
        }
    }
}

/// Trait for key types that can be stored in the keystore
pub trait KeyType: Sized + 'static {
    /// The secret key type
    type Secret: Clone + Serialize + serde::de::DeserializeOwned + Ord + Send + Sync;
    /// The public key type
    type Public: Clone + Serialize + serde::de::DeserializeOwned + Ord + Send + Sync;
    /// The signature type
    type Signature: Clone + Serialize + serde::de::DeserializeOwned + Ord + Send + Sync;

    /// Get the key type identifier
    fn key_type_id() -> KeyTypeId;

    /// Get a cryptographically secure random number generator
    #[cfg(feature = "std")]
    fn get_rng() -> impl gadget_std::CryptoRng + gadget_std::Rng {
        gadget_std::rand::thread_rng()
    }

    #[cfg(not(feature = "std"))]
    fn get_rng() -> impl gadget_std::CryptoRng + gadget_std::Rng {
        gadget_std::test_rng()
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
