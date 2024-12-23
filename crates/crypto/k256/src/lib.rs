#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
mod k256_ecdsa;
use k256::ecdsa::VerifyingKey;
pub use k256_ecdsa::*;

impl From<K256VerifyingKey> for VerifyingKey {
    fn from(key: K256VerifyingKey) -> Self {
        key.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use error::K256Error;
    use gadget_crypto_core::KeyType;
    use k256_ecdsa::{K256Ecdsa, K256Signature, K256SigningKey, K256VerifyingKey};

    mod k256_crypto_tests {
        use super::*;
        gadget_crypto_core::impl_crypto_tests!(K256Ecdsa, K256SigningKey, K256Signature);
    }

    #[test]
    fn test_key_generation_deterministic() {
        // Test seed-based generation is deterministic
        let seed = b"deterministic_test_seed";
        let key1 = K256Ecdsa::generate_with_seed(Some(seed)).unwrap();
        let key2 = K256Ecdsa::generate_with_seed(Some(seed)).unwrap();

        assert_eq!(key1, key2, "Same seed should produce identical keys");

        // Different seeds should produce different keys
        let different_seed = b"different_seed";
        let key3 = K256Ecdsa::generate_with_seed(Some(different_seed)).unwrap();
        assert_ne!(key1, key3, "Different seeds should produce different keys");
    }

    #[test]
    fn test_signature_verification_edge_cases() {
        let seed = b"test_signature_verification";
        let mut secret = K256Ecdsa::generate_with_seed(Some(seed)).unwrap();
        let public = K256Ecdsa::public_from_secret(&secret);

        // Test empty message
        let empty_msg = vec![];
        let empty_sig = K256Ecdsa::sign_with_secret(&mut secret, &empty_msg).unwrap();
        assert!(K256Ecdsa::verify(&public, &empty_msg, &empty_sig));

        // Test large message
        let large_msg = vec![0xFF; 1_000_000];
        let large_sig = K256Ecdsa::sign_with_secret(&mut secret, &large_msg).unwrap();
        assert!(K256Ecdsa::verify(&public, &large_msg, &large_sig));

        // Test message modification
        let msg = b"original message".to_vec();
        let sig = K256Ecdsa::sign_with_secret(&mut secret, &msg).unwrap();
        let mut modified_msg = msg.clone();
        modified_msg[0] ^= 1;
        assert!(!K256Ecdsa::verify(&public, &modified_msg, &sig));
    }

    #[test]
    fn test_key_reuse() {
        let seed = b"key_reuse_test";
        let mut secret = K256Ecdsa::generate_with_seed(Some(seed)).unwrap();
        let public = K256Ecdsa::public_from_secret(&secret);

        // Sign multiple messages with the same key
        let messages = vec![
            b"message1".to_vec(),
            b"message2".to_vec(),
            b"message3".to_vec(),
        ];

        for msg in &messages {
            let sig = K256Ecdsa::sign_with_secret(&mut secret, msg).unwrap();
            assert!(K256Ecdsa::verify(&public, msg, &sig));
        }
    }

    #[test]
    fn test_signature_malleability() {
        let seed = b"malleability_test";
        let mut secret = K256Ecdsa::generate_with_seed(Some(seed)).unwrap();
        let public = K256Ecdsa::public_from_secret(&secret);
        let message = b"test message".to_vec();

        let signature = K256Ecdsa::sign_with_secret(&mut secret, &message).unwrap();

        // Try to modify signature
        let mut modified_sig = signature.clone();
        let mut bytes = modified_sig.0.to_bytes().to_vec();
        bytes[0] ^= 1; // Flip one bit
        modified_sig = K256Signature::from_bytes(&bytes).unwrap();
        assert!(!K256Ecdsa::verify(&public, &message, &modified_sig));
    }

    #[test]
    fn test_cross_key_verification() {
        let seed1 = b"key1";
        let seed2 = b"key2";
        let mut secret1 = K256Ecdsa::generate_with_seed(Some(seed1)).unwrap();
        let mut secret2 = K256Ecdsa::generate_with_seed(Some(seed2)).unwrap();
        let public1 = K256Ecdsa::public_from_secret(&secret1);
        let public2 = K256Ecdsa::public_from_secret(&secret2);

        let message = b"test message".to_vec();
        let sig1 = K256Ecdsa::sign_with_secret(&mut secret1, &message).unwrap();
        let sig2 = K256Ecdsa::sign_with_secret(&mut secret2, &message).unwrap();

        // Verify correct signatures
        assert!(K256Ecdsa::verify(&public1, &message, &sig1));
        assert!(K256Ecdsa::verify(&public2, &message, &sig2));

        // Cross verification should fail
        assert!(!K256Ecdsa::verify(&public1, &message, &sig2));
        assert!(!K256Ecdsa::verify(&public2, &message, &sig1));
    }

    #[test]
    fn test_boundary_conditions() {
        let seed = b"boundary_test";
        let mut secret = K256Ecdsa::generate_with_seed(Some(seed)).unwrap();
        let public = K256Ecdsa::public_from_secret(&secret);

        // Test with various message sizes
        let test_sizes = [0, 1, 32, 64, 128, 256, 1024, 4096];

        for size in test_sizes.iter() {
            let message = vec![0xAA; *size];
            let signature = K256Ecdsa::sign_with_secret(&mut secret, &message).unwrap();
            assert!(K256Ecdsa::verify(&public, &message, &signature));
        }
    }

    #[test]
    fn test_key_serialization() {
        // Test seed that's too long
        let long_seed = vec![0xFF; 33];
        let result = K256Ecdsa::generate_with_seed(Some(&long_seed));
        assert!(result.is_err());
        match result {
            Err(K256Error::InvalidSeed(msg)) => {
                assert_eq!(msg, "Seed must not exceed 32 bytes");
            }
            _ => panic!("Expected InvalidSeed error"),
        }

        let seed = b"serialization_test";
        let secret = K256Ecdsa::generate_with_seed(Some(seed)).unwrap();
        let public = K256Ecdsa::public_from_secret(&secret);

        // Test public key serialization
        let public_bytes = public.to_bytes();
        let recovered_public = K256VerifyingKey::from_bytes(&public_bytes).unwrap();
        assert_eq!(public, recovered_public);

        // Test signature serialization
        let message = b"test message".to_vec();
        let mut secret_clone = secret.clone();
        let signature = K256Ecdsa::sign_with_secret(&mut secret_clone, &message).unwrap();

        let sig_bytes = signature.to_bytes();
        let recovered_sig = K256Signature::from_bytes(&sig_bytes).unwrap();
        assert_eq!(signature, recovered_sig);
    }

    #[test]
    fn test_invalid_key_deserialization() {
        // Test with wrong length
        let short_bytes = vec![0xFF; 31];
        assert!(K256VerifyingKey::from_bytes(&short_bytes).is_err());

        let long_bytes = vec![0xFF; 65];
        assert!(K256VerifyingKey::from_bytes(&long_bytes).is_err());
    }

    #[test]
    fn test_concurrent_key_usage() {
        use gadget_std::sync::Arc;
        use gadget_std::thread;

        let seed = b"concurrent_test";
        let secret = Arc::new(K256Ecdsa::generate_with_seed(Some(seed)).unwrap());
        let public = K256Ecdsa::public_from_secret(&secret);

        let mut handles = vec![];

        // Create multiple threads signing different messages
        for i in 0..10 {
            let secret = Arc::clone(&secret);
            let handle = thread::spawn(move || {
                let message = format!("message {}", i).into_bytes();
                let mut secret = (*secret).clone();
                let signature = K256Ecdsa::sign_with_secret(&mut secret, &message).unwrap();
                (message, signature)
            });
            handles.push(handle);
        }

        // Verify all signatures
        for handle in handles {
            let (message, signature) = handle.join().unwrap();
            assert!(K256Ecdsa::verify(&public, &message, &signature));
        }
    }

    #[test]
    fn test_low_s_normalization() {
        let seed = b"low_s_test";
        let mut secret = K256Ecdsa::generate_with_seed(Some(seed)).unwrap();
        let message = b"test message".to_vec();

        let signature = K256Ecdsa::sign_with_secret(&mut secret, &message).unwrap();

        // Verify that signature has low S value
        let sig_bytes = signature.to_bytes();
        let s_bytes = &sig_bytes[32..64];
        let high_bit = s_bytes[0] & 0x80;
        assert_eq!(high_bit, 0, "S value should be normalized to low S");
    }
}
