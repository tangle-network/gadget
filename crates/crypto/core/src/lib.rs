#![cfg_attr(not(feature = "std"), no_std)]

use gadget_std::string::String;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum KeyTypeId {
    #[cfg(feature = "bn254")]
    ArkBn254,
    #[cfg(feature = "k256")]
    K256Ecdsa,
    #[cfg(feature = "sr25519-schnorrkel")]
    SchnorrkelSr25519,
    #[cfg(feature = "bls")]
    W3fBls381,
    #[cfg(feature = "bls")]
    W3fBls377,
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
    pub const ENABLED: &'static [Self] = &[
        #[cfg(feature = "bn254")]
        Self::ArkBn254,
        #[cfg(feature = "k256")]
        Self::K256Ecdsa,
        #[cfg(feature = "sr25519-schnorrkel")]
        Self::SchnorrkelSr25519,
        #[cfg(feature = "bls")]
        Self::W3fBls377,
        #[cfg(feature = "bls")]
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
            #[cfg(feature = "k256")]
            Self::K256Ecdsa => "k256-ecdsa",
            #[cfg(feature = "sr25519-schnorrkel")]
            Self::SchnorrkelSr25519 => "schnorrkel-sr25519",
            #[cfg(feature = "bls")]
            Self::W3fBls381 => "w3f-bls381",
            #[cfg(feature = "bls")]
            Self::W3fBls377 => "w3f-bls377",
            #[cfg(feature = "zebra")]
            Self::ZebraEd25519 => "zebra-ed25519",
            #[cfg(feature = "tangle")]
            Self::SpBls377 => "sp-bls377",
            #[cfg(feature = "tangle")]
            Self::SpBls381 => "sp-bls381",
            #[cfg(feature = "tangle")]
            Self::SpEcdsa => "sp-ecdsa",
            #[cfg(feature = "tangle")]
            Self::SpEd25519 => "sp-ed25519",
            #[cfg(feature = "tangle")]
            Self::SpSr25519 => "sp-sr25519",
            #[cfg(all(
                not(feature = "bn254"),
                not(feature = "k256"),
                not(feature = "sr25519-schnorrkel"),
                not(feature = "bls"),
                not(feature = "zebra"),
                not(feature = "tangle")
            ))]
            _ => unreachable!("All possible variants are feature-gated"),
        }
    }
}

/// Trait for key types that can be stored in the keystore
pub trait KeyType: Sized + 'static {
    type Secret: Clone + Serialize + for<'de> Deserialize<'de> + Ord + Send + Sync;
    type Public: Clone + Serialize + for<'de> Deserialize<'de> + Ord + Send + Sync;
    type Signature: Clone + Serialize + for<'de> Deserialize<'de> + Ord + Send + Sync;
    type Error: Clone + Send + Sync;

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

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret, Self::Error>;
    fn generate_with_string(secret: String) -> Result<Self::Secret, Self::Error>;
    fn public_from_secret(secret: &Self::Secret) -> Self::Public;
    fn sign_with_secret(
        secret: &mut Self::Secret,
        msg: &[u8],
    ) -> Result<Self::Signature, Self::Error>;
    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature, Self::Error>;
    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool;
}
