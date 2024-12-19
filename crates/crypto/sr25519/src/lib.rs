#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
mod schnorrkel_sr25519;
pub use schnorrkel_sr25519::*;

#[cfg(test)]
mod tests {
    use super::*;
    use gadget_crypto_core::KeyType;

    mod sr25519_crypto_tests {
        use super::*;
        gadget_crypto_core::impl_crypto_tests!(
            SchnorrkelSr25519,
            SchnorrkelSecret,
            SchnorrkelSignature
        );
    }

    #[test]
    fn test_key_generation_deterministic() {
        // Test seed-based generation is deterministic
        let seed = b"deterministic_test_seed";
        let key1 = SchnorrkelSr25519::generate_with_seed(Some(seed)).unwrap();
        let key2 = SchnorrkelSr25519::generate_with_seed(Some(seed)).unwrap();

        assert_eq!(key1, key2, "Same seed should produce identical keys");

        // Different seeds should produce different keys
        let different_seed = b"different_seed";
        let key3 = SchnorrkelSr25519::generate_with_seed(Some(different_seed)).unwrap();
        assert_ne!(key1, key3, "Different seeds should produce different keys");
    }

    #[test]
    fn test_sr25519_pair_serialization() {
        // Create a known seed
        let seed: [u8; 32] = [1u8; 32];

        // Create the original pair
        let original_pair = SchnorrkelSr25519::generate_with_seed(Some(&seed)).unwrap();

        // Serialize and deserialize the pair directly
        let serialized = serde_json::to_vec(&original_pair).unwrap();
        let deserialized_pair: SchnorrkelSecret =
            serde_json::from_slice(&serialized).expect("Failed to deserialize pair");

        // Verify the pairs match
        assert_eq!(
            original_pair.0.to_bytes(),
            deserialized_pair.0.to_bytes(),
            "Deserialized pair doesn't match original"
        );
    }

    #[test]
    fn test_signature_verification_edge_cases() {
        let seed = b"test_signature_verification";
        let mut secret = SchnorrkelSr25519::generate_with_seed(Some(seed)).unwrap();
        let public = SchnorrkelSr25519::public_from_secret(&secret);

        // Test empty message
        let empty_msg = vec![];
        let empty_sig = SchnorrkelSr25519::sign_with_secret(&mut secret, &empty_msg).unwrap();
        assert!(SchnorrkelSr25519::verify(&public, &empty_msg, &empty_sig));

        // Test large message
        let large_msg = vec![0xFF; 1_000_000];
        let large_sig = SchnorrkelSr25519::sign_with_secret(&mut secret, &large_msg).unwrap();
        assert!(SchnorrkelSr25519::verify(&public, &large_msg, &large_sig));

        // Test message modification
        let msg = b"original message".to_vec();
        let sig = SchnorrkelSr25519::sign_with_secret(&mut secret, &msg).unwrap();
        let mut modified_msg = msg.clone();
        modified_msg[0] ^= 1;
        assert!(!SchnorrkelSr25519::verify(&public, &modified_msg, &sig));
    }

    #[test]
    fn test_cross_key_verification() {
        let seed1 = b"key1";
        let seed2 = b"key2";
        let mut secret1 = SchnorrkelSr25519::generate_with_seed(Some(seed1)).unwrap();
        let mut secret2 = SchnorrkelSr25519::generate_with_seed(Some(seed2)).unwrap();
        let public1 = SchnorrkelSr25519::public_from_secret(&secret1);
        let public2 = SchnorrkelSr25519::public_from_secret(&secret2);

        let message = b"test message".to_vec();
        let sig1 = SchnorrkelSr25519::sign_with_secret(&mut secret1, &message).unwrap();
        let sig2 = SchnorrkelSr25519::sign_with_secret(&mut secret2, &message).unwrap();

        // Verify correct signatures
        assert!(SchnorrkelSr25519::verify(&public1, &message, &sig1));
        assert!(SchnorrkelSr25519::verify(&public2, &message, &sig2));

        // Cross verification should fail
        assert!(!SchnorrkelSr25519::verify(&public1, &message, &sig2));
        assert!(!SchnorrkelSr25519::verify(&public2, &message, &sig1));
    }

    #[test]
    fn test_concurrent_key_usage() {
        use gadget_std::sync::Arc;
        use gadget_std::thread;

        let seed = b"concurrent_test";
        let secret = Arc::new(SchnorrkelSr25519::generate_with_seed(Some(seed)).unwrap());
        let public = SchnorrkelSr25519::public_from_secret(&secret);

        let mut handles = vec![];

        // Create multiple threads signing different messages
        for i in 0..10 {
            let secret = Arc::clone(&secret);
            let handle = thread::spawn(move || {
                let message = format!("message {}", i).into_bytes();
                let mut secret = (*secret).clone();
                let signature = SchnorrkelSr25519::sign_with_secret(&mut secret, &message).unwrap();
                (message, signature)
            });
            handles.push(handle);
        }

        // Verify all signatures
        for handle in handles {
            let (message, signature) = handle.join().unwrap();
            assert!(SchnorrkelSr25519::verify(&public, &message, &signature));
        }
    }
}
