use super::*;
use crate::{ArkBlsBn254, ArkBlsBn254Public, ArkBlsBn254Secret, ArkBlsBn254Signature};
use ark_ff::UniformRand;
use gadget_crypto_core::KeyType;
use gadget_crypto_hashing::keccak_256;
use gadget_std::string::ToString;

// Helper function to generate test message
fn test_message() -> Vec<u8> {
    b"test message".to_vec()
}

mod ark_bn254_crypto_tests {
    use super::*;
    gadget_crypto_core::impl_crypto_tests!(ArkBlsBn254, ArkBlsBn254Secret, ArkBlsBn254Signature);
}

#[test]
fn test_key_generation() {
    // Test seed-based generation is deterministic
    let seed = "123456789";
    let secret1 = ArkBlsBn254::generate_with_seed(Some(seed.as_bytes())).unwrap();
    let public1 = ArkBlsBn254::public_from_secret(&secret1);

    let secret2 = ArkBlsBn254::generate_with_seed(Some(seed.as_bytes())).unwrap();
    let public2 = ArkBlsBn254::public_from_secret(&secret2);

    // Same seed should produce same keys
    assert_eq!(public1, public2);

    // Test string-based generation is deterministic
    let secret_str = secret1.0.to_string();
    let secret_from_str1 = ArkBlsBn254::generate_with_string(secret_str.clone()).unwrap();
    let public_from_str1 = ArkBlsBn254::public_from_secret(&secret_from_str1);

    let secret_from_str2 = ArkBlsBn254::generate_with_string(secret_str).unwrap();
    let public_from_str2 = ArkBlsBn254::public_from_secret(&secret_from_str2);

    // Same string should produce same keys
    assert_eq!(public_from_str1, public_from_str2);
}

#[test]
fn test_signing_and_verification() {
    let seed = "test_signing";
    let mut secret = ArkBlsBn254::generate_with_seed(Some(seed.as_bytes())).unwrap();
    let public = ArkBlsBn254::public_from_secret(&secret);
    let message = test_message();

    // Test normal signing
    let signature = ArkBlsBn254::sign_with_secret(&mut secret, &message).unwrap();
    assert!(ArkBlsBn254::verify(&public, &message, &signature));

    // Test pre-hashed signing
    let hashed_msg = keccak_256(&message);
    let signature_pre_hashed =
        ArkBlsBn254::sign_with_secret_pre_hashed(&mut secret, &hashed_msg).unwrap();
    assert!(ArkBlsBn254::verify(
        &public,
        &hashed_msg,
        &signature_pre_hashed
    ));

    // Test invalid signature
    let wrong_message = b"wrong message".to_vec();
    assert!(!ArkBlsBn254::verify(&public, &wrong_message, &signature));
}

#[test]
fn test_serialization() {
    let seed = "test_serialization";
    let secret = ArkBlsBn254::generate_with_seed(Some(seed.as_bytes())).unwrap();
    let public = ArkBlsBn254::public_from_secret(&secret);

    // Test public key serialization
    let public_bytes = to_bytes(public.0);
    let public_deserialized = ArkBlsBn254Public(from_bytes(&public_bytes));
    assert_eq!(public, public_deserialized);

    // Test secret key serialization
    let secret_bytes = to_bytes(secret.0);
    let secret_deserialized = ArkBlsBn254Secret(from_bytes(&secret_bytes));
    assert_eq!(secret, secret_deserialized);

    // Test signature serialization
    let message = test_message();
    let mut secret_for_sig = secret.clone();
    let signature = ArkBlsBn254::sign_with_secret(&mut secret_for_sig, &message).unwrap();
    let signature_bytes = to_bytes(signature.0);
    let signature_deserialized = ArkBlsBn254Signature(from_bytes(&signature_bytes));
    assert_eq!(signature, signature_deserialized);
}

#[test]
fn test_error_handling() {
    // Test invalid string for key generation (not a valid field element)
    let result = ArkBlsBn254::generate_with_string("invalid field element".to_string());
    assert!(result.is_err());

    // Test empty string
    let result = ArkBlsBn254::generate_with_string("".to_string());
    assert!(result.is_err());

    // Test invalid points for signature verification
    let mut rng = gadget_std::test_rng();
    let invalid_signature = G1Affine::rand(&mut rng);
    let invalid_public = G2Affine::rand(&mut rng);
    let message = test_message();

    // Test with invalid signature point
    assert!(!verify(invalid_public, &message, invalid_signature));

    // Test signature generation with edge cases
    let seed = "test_edge_cases";
    let mut secret = ArkBlsBn254::generate_with_seed(Some(seed.as_bytes())).unwrap();

    // Test empty message
    let empty_msg: Vec<u8> = vec![];
    let sig_empty = ArkBlsBn254::sign_with_secret(&mut secret, &empty_msg).unwrap();
    assert!(ArkBlsBn254::verify(
        &ArkBlsBn254::public_from_secret(&secret),
        &empty_msg,
        &sig_empty
    ));

    // Test with all zero message
    let zero_msg = vec![0u8; 32];
    let sig_zero = ArkBlsBn254::sign_with_secret(&mut secret, &zero_msg).unwrap();
    assert!(ArkBlsBn254::verify(
        &ArkBlsBn254::public_from_secret(&secret),
        &zero_msg,
        &sig_zero
    ));

    // Test with all ones message
    let ones_msg = vec![0xFF; 32];
    let sig_ones = ArkBlsBn254::sign_with_secret(&mut secret, &ones_msg).unwrap();
    assert!(ArkBlsBn254::verify(
        &ArkBlsBn254::public_from_secret(&secret),
        &ones_msg,
        &sig_ones
    ));
}

#[test]
fn test_hash_to_curve() {
    let message1 = b"test message 1";
    let message2 = b"test message 2";

    // Test deterministic hashing
    let point1a = hash_to_curve(message1);
    let point1b = hash_to_curve(message1);
    let point2 = hash_to_curve(message2);

    // Same message should hash to same point
    assert_eq!(point1a, point1b);
    // Different messages should hash to different points
    assert_ne!(point1a, point2);

    // Test point validity
    assert!(point1a.is_on_curve());
    assert!(point1a.is_in_correct_subgroup_assuming_on_curve());
    assert!(point2.is_on_curve());
    assert!(point2.is_in_correct_subgroup_assuming_on_curve());
}

#[test]
fn test_hash_to_curve_edge_cases() {
    // Test empty input
    let empty = b"";
    let point_empty = hash_to_curve(empty);
    assert!(point_empty.is_on_curve());
    assert!(point_empty.is_in_correct_subgroup_assuming_on_curve());

    // Test large input
    let large_input = vec![0xFF; 1_000_000];
    let point_large = hash_to_curve(&large_input);
    assert!(point_large.is_on_curve());
    assert!(point_large.is_in_correct_subgroup_assuming_on_curve());

    // Test all zeros
    let zeros = vec![0u8; 32];
    let point_zeros = hash_to_curve(&zeros);
    assert!(point_zeros.is_on_curve());
    assert!(point_zeros.is_in_correct_subgroup_assuming_on_curve());

    // Test all ones
    let ones = vec![0xFF; 32];
    let point_ones = hash_to_curve(&ones);
    assert!(point_ones.is_on_curve());
    assert!(point_ones.is_in_correct_subgroup_assuming_on_curve());

    // Verify different inputs produce different points
    assert_ne!(point_empty, point_large);
    assert_ne!(point_zeros, point_ones);
}

#[test]
fn test_serialization_edge_cases() {
    // Test serialization with edge case keys
    let seed = "serialization_edge";
    let secret = ArkBlsBn254::generate_with_seed(Some(seed.as_bytes())).unwrap();
    let public = ArkBlsBn254::public_from_secret(&secret);

    // Test public key serialization/deserialization
    let public_bytes = to_bytes(public.0);
    let public_deserialized = ArkBlsBn254Public(from_bytes(&public_bytes));
    assert_eq!(public, public_deserialized);

    // Test secret key serialization/deserialization
    let secret_bytes = to_bytes(secret.0);
    let secret_deserialized = ArkBlsBn254Secret(from_bytes(&secret_bytes));
    assert_eq!(secret, secret_deserialized);

    // Test signature serialization
    let message = vec![0u8; 32];
    let mut secret_clone = secret.clone();
    let signature = ArkBlsBn254::sign_with_secret(&mut secret_clone, &message).unwrap();

    // Test normal serialization/deserialization
    let sig_bytes = to_bytes(signature.0);
    let sig_deserialized = ArkBlsBn254Signature(from_bytes(&sig_bytes));
    assert_eq!(signature, sig_deserialized);

    // Test with corrupted signature bytes (if possible to deserialize)
    if sig_bytes.is_empty() {
        let mut corrupted_sig_bytes = sig_bytes.clone();
        corrupted_sig_bytes[0] ^= 1; // Flip one bit in the first byte

        // Try to deserialize corrupted bytes - might fail or produce invalid signature
        if let Ok(corrupted_sig) =
            std::panic::catch_unwind(|| ArkBlsBn254Signature(from_bytes(&corrupted_sig_bytes)))
        {
            // If deserialization succeeded, signature should be different
            assert_ne!(signature, corrupted_sig);
            // Verification should fail
            assert!(!ArkBlsBn254::verify(&public, &message, &corrupted_sig));
        }
    }
}

#[test]
fn test_key_generation_edge_cases() {
    // Test with various problematic seeds
    let edge_cases = vec![
        "0",         // Minimal valid input
        "1",         // Another minimal valid input
        "123456789", // Numeric string
        "deadbeef",  // Hex-like string
        "test_seed", // Normal string
    ];

    for seed in edge_cases {
        // Test seed-based generation
        let result = ArkBlsBn254::generate_with_seed(Some(seed.as_bytes()));
        assert!(result.is_ok());

        let secret = result.unwrap();
        let public = ArkBlsBn254::public_from_secret(&secret);

        // Test basic signing with generated keys
        let msg = b"test";
        let mut secret_clone = secret.clone();
        let signature = ArkBlsBn254::sign_with_secret(&mut secret_clone, msg).unwrap();
        assert!(ArkBlsBn254::verify(&public, msg, &signature));
    }
}
