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
