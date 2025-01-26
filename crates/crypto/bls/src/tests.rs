use super::*;
use crate::{
    bls377::{W3fBls377, W3fBls377Public, W3fBls377Secret},
    bls381::{W3fBls381, W3fBls381Public, W3fBls381Secret},
};
use gadget_crypto_core::KeyType;
use gadget_std::string::ToString;

// Helper function to generate test message
fn test_message() -> Vec<u8> {
    b"test message".to_vec()
}

mod bls377_crypto_tests {
    use super::bls377::{W3fBls377, W3fBls377Secret, W3fBls377Signature};
    gadget_crypto_core::impl_crypto_tests!(W3fBls377, W3fBls377Secret, W3fBls377Signature);
}

mod bls381_crypto_tests {
    use super::bls381::{W3fBls381, W3fBls381Secret, W3fBls381Signature};
    gadget_crypto_core::impl_crypto_tests!(W3fBls381, W3fBls381Secret, W3fBls381Signature);
}

mod bls377_tests {
    use super::*;
    use ::w3f_bls::SerializableToBytes;
    use gadget_crypto_hashing::sha2_256;

    #[test]
    fn test_key_generation() {
        // Test seed-based generation is deterministic
        let seed = [0u8; 32];
        let secret1 = W3fBls377::generate_with_seed(Some(&seed)).unwrap();
        let public1 = W3fBls377::public_from_secret(&secret1);

        let secret2 = W3fBls377::generate_with_seed(Some(&seed)).unwrap();
        let public2 = W3fBls377::public_from_secret(&secret2);

        // Same seed should produce same keys
        assert_eq!(public1, public2);

        let secret1_hex = hex::encode(secret1.0.to_bytes());
        let secret2_hex = hex::encode(secret2.0.to_bytes());

        // Test string-based generation is deterministic
        let secret_from_str1 = W3fBls377::generate_with_string(secret1_hex).unwrap();
        let public_from_str1 = W3fBls377::public_from_secret(&secret_from_str1);

        let secret_from_str2 = W3fBls377::generate_with_string(secret2_hex).unwrap();
        let public_from_str2 = W3fBls377::public_from_secret(&secret_from_str2);

        // Same string should produce same keys
        assert_eq!(public_from_str1, public_from_str2);
    }

    #[test]
    fn test_signing_and_verification() {
        let seed = [0u8; 32];
        let mut secret = W3fBls377::generate_with_seed(Some(&seed)).unwrap();
        let public = W3fBls377::public_from_secret(&secret);
        let message = test_message();

        // Test normal signing
        let signature = W3fBls377::sign_with_secret(&mut secret, &message).unwrap();
        assert!(W3fBls377::verify(&public, &message, &signature));

        // Test pre-hashed signing
        let hashed_msg = sha2_256(&message);
        let signature_pre_hashed =
            W3fBls377::sign_with_secret_pre_hashed(&mut secret, &hashed_msg).unwrap();
        assert!(W3fBls377::verify(
            &public,
            &hashed_msg,
            &signature_pre_hashed
        ));

        // Test invalid signature
        let wrong_message = b"wrong message".to_vec();
        assert!(!W3fBls377::verify(&public, &wrong_message, &signature));
    }

    #[test]
    fn test_serialization() {
        let seed = [0u8; 32];
        let secret = W3fBls377::generate_with_seed(Some(&seed)).unwrap();
        let public = W3fBls377::public_from_secret(&secret);

        // Test public key serialization
        let public_bytes = to_bytes(public.0);
        let public_deserialized: W3fBls377Public = W3fBls377Public(from_bytes(&public_bytes));
        assert_eq!(public, public_deserialized);

        // Test secret key serialization
        let secret_bytes = to_bytes(secret.0.clone());
        let secret_deserialized: W3fBls377Secret = W3fBls377Secret(from_bytes(&secret_bytes));
        assert_eq!(secret, secret_deserialized);
    }

    #[test]
    fn test_error_handling() {
        // Test invalid seed
        let result = W3fBls377::generate_with_string("invalid hex".to_string());
        assert!(result.is_err());

        // Test empty seed
        let result = W3fBls377::generate_with_string("".to_string());
        assert!(result.is_err());
    }
}

mod bls381_tests {
    use super::*;
    use ::w3f_bls::SerializableToBytes;
    use gadget_crypto_hashing::sha2_256;

    #[test]
    fn test_key_generation() {
        // Test seed-based generation is deterministic
        let seed = [0u8; 32];
        let secret1 = W3fBls381::generate_with_seed(Some(&seed)).unwrap();
        let public1 = W3fBls381::public_from_secret(&secret1);

        let secret2 = W3fBls381::generate_with_seed(Some(&seed)).unwrap();
        let public2 = W3fBls381::public_from_secret(&secret2);

        // Same seed should produce same keys
        assert_eq!(public1, public2);

        let secret1_hex = hex::encode(secret1.0.to_bytes());
        let secret2_hex = hex::encode(secret2.0.to_bytes());

        // Test string-based generation is deterministic
        let secret_from_str1 = W3fBls381::generate_with_string(secret1_hex).unwrap();
        let public_from_str1 = W3fBls381::public_from_secret(&secret_from_str1);

        let secret_from_str2 = W3fBls381::generate_with_string(secret2_hex).unwrap();
        let public_from_str2 = W3fBls381::public_from_secret(&secret_from_str2);

        // Same string should produce same keys
        assert_eq!(public_from_str1, public_from_str2);
    }

    #[test]
    fn test_signing_and_verification() {
        let seed = [0u8; 32];
        let mut secret = W3fBls381::generate_with_seed(Some(&seed)).unwrap();
        let public = W3fBls381::public_from_secret(&secret);
        let message = test_message();

        // Test normal signing
        let signature = W3fBls381::sign_with_secret(&mut secret, &message).unwrap();
        assert!(W3fBls381::verify(&public, &message, &signature));

        // Test pre-hashed signing
        let hashed_msg = sha2_256(&message);
        let signature_pre_hashed =
            W3fBls381::sign_with_secret_pre_hashed(&mut secret, &hashed_msg).unwrap();
        // For pre-hashed messages, we need to hash the verification message as well
        assert!(W3fBls381::verify(
            &public,
            &hashed_msg,
            &signature_pre_hashed
        ));

        // Test invalid signature
        let wrong_message = b"wrong message".to_vec();
        assert!(!W3fBls381::verify(&public, &wrong_message, &signature));
    }

    #[test]
    fn test_serialization() {
        let seed = b"test seed for serialization";
        let secret = W3fBls381::generate_with_seed(Some(seed)).unwrap();
        let public = W3fBls381::public_from_secret(&secret);

        // Test public key serialization
        let public_bytes = to_bytes(public.0);
        let public_deserialized: W3fBls381Public = W3fBls381Public(from_bytes(&public_bytes));
        assert_eq!(public, public_deserialized);

        // Test secret key serialization
        let secret_bytes = to_bytes(secret.0.clone());
        let secret_deserialized: W3fBls381Secret = W3fBls381Secret(from_bytes(&secret_bytes));
        assert_eq!(secret, secret_deserialized);
    }

    #[test]
    fn test_error_handling() {
        // Test invalid seed
        let result = W3fBls381::generate_with_string("invalid hex".to_string());
        assert!(result.is_err());

        // Test empty seed
        let result = W3fBls381::generate_with_string("".to_string());
        assert!(result.is_err());
    }
}
