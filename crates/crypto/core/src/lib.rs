#![cfg_attr(not(feature = "std"), no_std)]

use gadget_std::string::String;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum KeyTypeId {
    #[cfg(feature = "bn254")]
    Bn254,
    #[cfg(any(feature = "k256", feature = "tangle"))]
    Ecdsa,
    #[cfg(any(feature = "sr25519-schnorrkel", feature = "tangle"))]
    Sr25519,
    #[cfg(any(feature = "bls", feature = "tangle"))]
    Bls381,
    #[cfg(any(feature = "bls", feature = "tangle"))]
    Bls377,
    #[cfg(any(feature = "zebra", feature = "tangle"))]
    Ed25519,
}

impl KeyTypeId {
    pub const ENABLED: &'static [Self] = &[
        #[cfg(feature = "bn254")]
        Self::Bn254,
        #[cfg(any(feature = "k256", feature = "tangle"))]
        Self::Ecdsa,
        #[cfg(any(feature = "sr25519-schnorrkel", feature = "tangle"))]
        Self::Sr25519,
        #[cfg(any(feature = "bls", feature = "tangle"))]
        Self::Bls381,
        #[cfg(any(feature = "bls", feature = "tangle"))]
        Self::Bls377,
        #[cfg(any(feature = "zebra", feature = "tangle"))]
        Self::Ed25519,
    ];

    pub fn name(&self) -> &'static str {
        match *self {
            #[cfg(feature = "bn254")]
            Self::Bn254 => "bn254",
            #[cfg(any(feature = "k256", feature = "tangle"))]
            Self::Ecdsa => "ecdsa",
            #[cfg(any(feature = "sr25519-schnorrkel", feature = "tangle"))]
            Self::Sr25519 => "sr25519",
            #[cfg(any(feature = "bls", feature = "tangle"))]
            Self::Bls381 => "bls381",
            #[cfg(any(feature = "bls", feature = "tangle"))]
            Self::Bls377 => "bls377",
            #[cfg(any(feature = "zebra", feature = "tangle"))]
            Self::Ed25519 => "ed25519",
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

pub trait KeyEncoding: Sized {
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self, serde::de::value::Error>;
}

/// Trait for key types that can be stored in the keystore
pub trait KeyType: Sized + 'static {
    type Secret: Clone + Serialize + for<'de> Deserialize<'de> + Ord + Send + Sync + KeyEncoding;
    type Public: Clone + Serialize + for<'de> Deserialize<'de> + Ord + Send + Sync + KeyEncoding;
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

#[cfg(feature = "clap")]
impl clap::ValueEnum for KeyTypeId {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Sr25519,
            Self::Ed25519,
            Self::Ecdsa,
            Self::Bls381,
            Self::Bn254,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::Sr25519 => {
                clap::builder::PossibleValue::new("sr25519").help("Schnorrkel/Ristretto x25519")
            }
            Self::Ed25519 => {
                clap::builder::PossibleValue::new("ed25519").help("Edwards Curve 25519")
            }
            Self::Ecdsa => clap::builder::PossibleValue::new("ecdsa")
                .help("Elliptic Curve Digital Signature Algorithm"),
            Self::Bls381 => {
                clap::builder::PossibleValue::new("bls381").help("Boneh-Lynn-Shacham on BLS12-381")
            }
            Self::Bn254 => {
                clap::builder::PossibleValue::new("blsbn254").help("Boneh-Lynn-Shacham on BN254")
            }
            _ => return None,
        })
    }
}

#[macro_export]
macro_rules! impl_crypto_tests {
    ($crypto_type:ty, $signing_key:ty, $signature:ty) => {
        use $crate::KeyType;
        #[test]
        fn test_key_generation() {
            // Test random key generation
            let secret = <$crypto_type>::generate_with_seed(None).unwrap();
            let _public = <$crypto_type>::public_from_secret(&secret);
        }

        #[test]
        fn test_signing_and_verification() {
            let mut secret = <$crypto_type>::generate_with_seed(None).unwrap();
            let public = <$crypto_type>::public_from_secret(&secret);

            // Test normal signing
            let message = b"Hello, world!";
            let signature = <$crypto_type>::sign_with_secret(&mut secret, message).unwrap();
            assert!(
                <$crypto_type>::verify(&public, message, &signature),
                "Signature verification failed"
            );

            // Test pre-hashed signing
            let hashed_msg = [42u8; 32];
            let signature =
                <$crypto_type>::sign_with_secret_pre_hashed(&mut secret, &hashed_msg).unwrap();

            // Verify with wrong message should fail
            let wrong_message = b"Wrong message";
            assert!(
                !<$crypto_type>::verify(&public, wrong_message, &signature),
                "Verification should fail with wrong message"
            );
        }

        #[test]
        fn test_key_serialization() {
            let secret = <$crypto_type>::generate_with_seed(None).unwrap();
            let public = <$crypto_type>::public_from_secret(&secret);

            // Test signing key serialization
            let serialized = serde_json::to_string(&secret).unwrap();
            let deserialized: $signing_key = serde_json::from_str(&serialized).unwrap();
            assert_eq!(
                secret, deserialized,
                "SigningKey serialization roundtrip failed"
            );

            // Test verifying key serialization
            let serialized = serde_json::to_string(&public).unwrap();
            let deserialized = serde_json::from_str(&serialized).unwrap();
            assert_eq!(
                public, deserialized,
                "VerifyingKey serialization roundtrip failed"
            );
        }

        #[test]
        fn test_signature_serialization() {
            let mut secret = <$crypto_type>::generate_with_seed(None).unwrap();
            let message = b"Test message";
            let signature = <$crypto_type>::sign_with_secret(&mut secret, message).unwrap();

            // Test signature serialization
            let serialized = serde_json::to_string(&signature).unwrap();
            let deserialized: $signature = serde_json::from_str(&serialized).unwrap();
            assert_eq!(
                signature, deserialized,
                "Signature serialization roundtrip failed"
            );
        }

        #[test]
        fn test_key_comparison() {
            let secret1 = <$crypto_type>::generate_with_seed(None).unwrap();
            let secret2 = <$crypto_type>::generate_with_seed(None).unwrap();
            let public1 = <$crypto_type>::public_from_secret(&secret1);
            let public2 = <$crypto_type>::public_from_secret(&secret2);

            // Test Ord implementation
            assert!(public1 != public2, "Different keys should not be equal");
            assert_eq!(public1.cmp(&public1), gadget_std::cmp::Ordering::Equal);

            // Verify consistency between PartialOrd and Ord
            assert_eq!(public1.partial_cmp(&public2), Some(public1.cmp(&public2)));
        }
    };
}
