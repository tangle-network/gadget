#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
mod zebra_ed25519;

pub use zebra_ed25519::*;

#[cfg(test)]
mod tests {
    use super::*;
    use gadget_crypto_core::{KeyEncoding, KeyType};

    mod ed25519_crypto_tests {
        use super::*;
        gadget_crypto_core::impl_crypto_tests!(Ed25519Zebra, Ed25519SigningKey, Ed25519Signature);
    }

    #[test]
    fn test_key_generation_deterministic() {
        // Test seed-based generation is deterministic
        let seed = b"deterministic_test_seed";
        let key1 = Ed25519Zebra::generate_with_seed(Some(seed)).unwrap();
        let key2 = Ed25519Zebra::generate_with_seed(Some(seed)).unwrap();

        assert_eq!(key1, key2, "Same seed should produce identical keys");

        // Different seeds should produce different keys
        let different_seed = b"different_seed";
        let key3 = Ed25519Zebra::generate_with_seed(Some(different_seed)).unwrap();
        assert_ne!(key1, key3, "Different seeds should produce different keys");
    }

    #[test]
    fn test_signature_verification_edge_cases() {
        let seed = b"test_signature_verification";
        let mut secret = Ed25519Zebra::generate_with_seed(Some(seed)).unwrap();
        let public = Ed25519Zebra::public_from_secret(&secret);

        // Test empty message
        let empty_msg = vec![];
        let empty_sig = Ed25519Zebra::sign_with_secret(&mut secret, &empty_msg).unwrap();
        assert!(Ed25519Zebra::verify(&public, &empty_msg, &empty_sig));

        // Test large message
        let large_msg = vec![0xFF; 1_000_000];
        let large_sig = Ed25519Zebra::sign_with_secret(&mut secret, &large_msg).unwrap();
        assert!(Ed25519Zebra::verify(&public, &large_msg, &large_sig));

        // Test message modification
        let msg = b"original message".to_vec();
        let sig = Ed25519Zebra::sign_with_secret(&mut secret, &msg).unwrap();
        let mut modified_msg = msg.clone();
        modified_msg[0] ^= 1;
        assert!(!Ed25519Zebra::verify(&public, &modified_msg, &sig));
    }

    #[test]
    fn test_key_reuse() {
        let seed = b"key_reuse_test";
        let mut secret = Ed25519Zebra::generate_with_seed(Some(seed)).unwrap();
        let public = Ed25519Zebra::public_from_secret(&secret);

        // Sign multiple messages with the same key
        let messages = vec![
            b"message1".to_vec(),
            b"message2".to_vec(),
            b"message3".to_vec(),
        ];

        for msg in &messages {
            let sig = Ed25519Zebra::sign_with_secret(&mut secret, msg).unwrap();
            assert!(Ed25519Zebra::verify(&public, msg, &sig));
        }
    }

    #[test]
    fn test_signature_malleability() {
        let seed = b"malleability_test";
        let mut secret = Ed25519Zebra::generate_with_seed(Some(seed)).unwrap();
        let public = Ed25519Zebra::public_from_secret(&secret);
        let message = b"test message".to_vec();

        let signature = Ed25519Zebra::sign_with_secret(&mut secret, &message).unwrap();

        // Try to modify signature
        let mut modified_sig = signature.clone();
        let mut bytes = modified_sig.0.to_bytes().to_vec();
        bytes[0] = if bytes[0] == 0 { 1 } else { 0 };
        modified_sig = Ed25519Signature::from_bytes(&bytes).unwrap();
        assert!(!Ed25519Zebra::verify(&public, &message, &modified_sig));
    }

    #[test]
    fn test_cross_key_verification() {
        let seed1 = b"key1";
        let seed2 = b"key2";
        let mut secret1 = Ed25519Zebra::generate_with_seed(Some(seed1)).unwrap();
        let mut secret2 = Ed25519Zebra::generate_with_seed(Some(seed2)).unwrap();
        let public1 = Ed25519Zebra::public_from_secret(&secret1);
        let public2 = Ed25519Zebra::public_from_secret(&secret2);

        let message = b"test message".to_vec();
        let sig1 = Ed25519Zebra::sign_with_secret(&mut secret1, &message).unwrap();
        let sig2 = Ed25519Zebra::sign_with_secret(&mut secret2, &message).unwrap();

        // Verify correct signatures
        assert!(Ed25519Zebra::verify(&public1, &message, &sig1));
        assert!(Ed25519Zebra::verify(&public2, &message, &sig2));

        // Cross verification should fail
        assert!(!Ed25519Zebra::verify(&public1, &message, &sig2));
        assert!(!Ed25519Zebra::verify(&public2, &message, &sig1));
    }

    #[test]
    fn test_boundary_conditions() {
        let seed = b"boundary_test";
        let mut secret = Ed25519Zebra::generate_with_seed(Some(seed)).unwrap();
        let public = Ed25519Zebra::public_from_secret(&secret);

        // Test with various message sizes
        let test_sizes = [0, 1, 32, 64, 128, 256, 1024, 4096];

        for size in test_sizes.iter() {
            let message = vec![0xAA; *size];
            let signature = Ed25519Zebra::sign_with_secret(&mut secret, &message).unwrap();
            assert!(Ed25519Zebra::verify(&public, &message, &signature));
        }
    }

    #[test]
    fn test_key_serialization() {
        let seed = b"serialization_test";
        let secret = Ed25519Zebra::generate_with_seed(Some(seed)).unwrap();
        let public = Ed25519Zebra::public_from_secret(&secret);

        // Test public key serialization
        let public_bytes = public.to_bytes();
        let recovered_public = Ed25519VerificationKey::from_bytes(&public_bytes).unwrap();
        assert_eq!(public, recovered_public);

        // Test signature serialization
        let message = b"test message".to_vec();
        let mut secret_clone = secret.clone();
        let signature = Ed25519Zebra::sign_with_secret(&mut secret_clone, &message).unwrap();

        let sig_bytes = signature.to_bytes();
        let recovered_sig = Ed25519Signature::from_bytes(&sig_bytes).unwrap();
        assert_eq!(signature, recovered_sig);
    }

    #[test]
    fn test_invalid_key_deserialization() {
        // Test with wrong length
        let short_bytes = vec![0xFF; 31];
        assert!(Ed25519VerificationKey::from_bytes(&short_bytes).is_err());

        let long_bytes = vec![0xFF; 33];
        assert!(Ed25519VerificationKey::from_bytes(&long_bytes).is_err());
    }

    #[test]
    fn test_concurrent_key_usage() {
        use gadget_std::sync::Arc;
        use gadget_std::thread;

        let seed = b"concurrent_test";
        let secret = Arc::new(Ed25519Zebra::generate_with_seed(Some(seed)).unwrap());
        let public = Ed25519Zebra::public_from_secret(&secret);

        let mut handles = vec![];

        // Create multiple threads signing different messages
        for i in 0..10 {
            let secret = Arc::clone(&secret);
            let handle = thread::spawn(move || {
                let message = format!("message {}", i).into_bytes();
                let mut secret = (*secret).clone();
                let signature = Ed25519Zebra::sign_with_secret(&mut secret, &message).unwrap();
                (message, signature)
            });
            handles.push(handle);
        }

        // Verify all signatures
        for handle in handles {
            let (message, signature) = handle.join().unwrap();
            assert!(Ed25519Zebra::verify(&public, &message, &signature));
        }
    }
}
