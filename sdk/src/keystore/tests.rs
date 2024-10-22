#![allow(dead_code, unused_imports)]
use crate::keystore::backend::fs::FilesystemKeystore;
use crate::keystore::backend::mem::InMemoryKeystore;
use crate::keystore::Backend;
use crate::keystore::KeystoreUriSanitizer;
use parking_lot::RawRwLock;
use std::path::PathBuf;

const KEYSTORE_PATH: &str = "./test_keys";

fn setup_mem_test_keystore() -> InMemoryKeystore<RawRwLock> {
    InMemoryKeystore::new()
}

fn setup_fs_test_keystore(key_type: &str) -> (FilesystemKeystore, String) {
    let key_path = format!("{KEYSTORE_PATH}-{key_type}");
    (
        FilesystemKeystore::open(key_path.clone()).unwrap(),
        key_path,
    )
}

#[test]
fn test_sanitize_file_paths() {
    let path = "file:///tmp/keystore";
    let sanitized_path = path.sanitize_file_path();
    assert_eq!(sanitized_path, PathBuf::from("/tmp/keystore"));
}

#[test]
fn test_sanitize_file_paths_with_single_slash() {
    let path = "file:/tmp/keystore";
    let sanitized_path = path.sanitize_file_path();
    assert_eq!(sanitized_path, PathBuf::from("/tmp/keystore"));
}

#[test]
fn test_sanitize_file_paths_with_double_slash() {
    let path = "file://tmp/keystore";
    let sanitized_path = path.sanitize_file_path();
    assert_eq!(sanitized_path, PathBuf::from("/tmp/keystore"));
}

#[test]
fn test_sanitize_file_paths_with_no_scheme() {
    let path = "/tmp/keystore";
    let sanitized_path = path.sanitize_file_path();
    assert_eq!(sanitized_path, PathBuf::from("/tmp/keystore"));
}

mod sr25519 {
    use super::*;
    use crate::keystore::sr25519::SIGNING_CTX;
    use crate::keystore::BackendExt;
    use schnorrkel::Keypair;
    use sp_core::Pair;

    #[test]
    #[cfg(all(feature = "getrandom", feature = "std"))]
    fn keys() {
        // In-Memory
        let mem_keystore = setup_mem_test_keystore();
        let mem_public = mem_keystore
            .sr25519_generate_new(None)
            .expect("Should generate key");
        // Read back the key from in-memory storage
        let mem_read = mem_keystore.sr25519_key().expect("Should read keys");
        assert_eq!(
            mem_public.to_bytes(),
            mem_read.public().0,
            "In-memory key should not be mutated in storage"
        );

        // Filesystem
        let (fs_keystore, path) = setup_fs_test_keystore("sr25519");
        let fs_public = fs_keystore
            .sr25519_generate_new(None)
            .expect("Should generate key");
        // Read back the key from filesystem storage
        let fs_read = fs_keystore.sr25519_key().expect("Should read keys");
        assert_eq!(
            fs_public.to_bytes(),
            fs_read.public().0,
            "Filesystem key should not be mutated in storage"
        );

        // Clean up filesystem keystore
        std::fs::remove_dir_all(path).expect("Failed to clean up test directory");
    }

    #[test]
    #[cfg(all(feature = "getrandom"))]
    fn signature() {
        let keystore = setup_mem_test_keystore();
        let public = keystore
            .sr25519_generate_new(None)
            .expect("Should generate key");
        let secret = keystore.expose_sr25519_secret(&public).unwrap().unwrap();
        let pair = Keypair { secret, public };
        let msg = b"test message";
        let sig = keystore
            .sr25519_sign(&public, msg)
            .expect("Should sign")
            .expect("Should have signature");
        let ctx = schnorrkel::signing_context(SIGNING_CTX);

        // Verify with correct message
        assert!(pair.public.verify(ctx.bytes(msg), &sig).is_ok());
        // Try to verify with wrong message
        let wrong_msg = b"wrong message";
        assert!(pair.public.verify(ctx.bytes(wrong_msg), &sig).is_err());
    }
}

mod ed25519 {
    use super::*;
    use crate::keystore::ed25519::Public;
    use crate::keystore::BackendExt;
    use sp_core::Pair;

    #[test]
    #[cfg(all(feature = "getrandom", feature = "std"))]
    fn keys() {
        // In-Memory
        let mem_keystore = setup_mem_test_keystore();
        let mem_public = mem_keystore
            .ed25519_generate_new(None)
            .expect("Should generate key");
        // Read back the key from in-memory storage
        let mem_read = mem_keystore.ed25519_key().expect("Should read keys");
        assert_eq!(
            mem_public.as_ref(),
            mem_read.public().0.as_ref(),
            "In-memory key should not be mutated in storage"
        );

        // Filesystem
        let (fs_keystore, path) = setup_fs_test_keystore("ed25519");
        let fs_public = fs_keystore
            .ed25519_generate_new(None)
            .expect("Should generate key");
        // Read back the key from filesystem storage
        let fs_read = fs_keystore.ed25519_key().expect("Should read keys");
        assert_eq!(
            fs_public.as_ref(),
            fs_read.public().0.as_ref(),
            "Filesystem key should not be mutated in storage"
        );

        // Clean up filesystem keystore
        std::fs::remove_dir_all(path).expect("Failed to clean up test directory");
    }

    #[test]
    #[cfg(all(feature = "getrandom"))]
    fn signature() {
        let keystore = setup_mem_test_keystore();
        let public = keystore
            .ed25519_generate_new(None)
            .expect("Should generate key");
        let secret = keystore.expose_ed25519_secret(&public).unwrap().unwrap();
        let verification_key = Public::from(&secret);
        let msg = b"test message";
        let sig = keystore
            .ed25519_sign(&public, msg)
            .expect("Should sign")
            .expect("Should have signature");

        // Verify with correct message
        assert!(verification_key.verify(&sig, msg).is_ok());

        // Try to verify with wrong message
        let wrong_msg = b"wrong message";
        assert!(verification_key.verify(&sig, wrong_msg).is_err());
    }
}

mod ecdsa {
    use super::*;
    use crate::keystore::BackendExt;
    use k256::ecdsa::signature::Verifier;
    use sp_core::Pair;

    #[test]
    #[cfg(all(feature = "getrandom", feature = "std"))]
    fn keys() {
        // In-Memory
        let mem_keystore = setup_mem_test_keystore();
        let mem_public = mem_keystore
            .ecdsa_generate_new(None)
            .expect("Should generate key");
        // Read back the key from in-memory storage
        let mem_read = mem_keystore.ecdsa_key().expect("Should read keys");
        assert_eq!(
            mem_public.to_sec1_bytes().as_ref(),
            mem_read.public().0,
            "In-memory key should not be mutated in storage"
        );

        // Filesystem
        let (fs_keystore, path) = setup_fs_test_keystore("ecdsa");
        let fs_public = fs_keystore
            .ecdsa_generate_new(None)
            .expect("Should generate key");
        // Read back the key from filesystem storage
        let fs_read = fs_keystore.ecdsa_key().expect("Should read keys");
        assert_eq!(
            fs_public.to_sec1_bytes().as_ref(),
            fs_read.public().0,
            "Filesystem key should not be mutated in storage"
        );

        // Clean up filesystem keystore
        std::fs::remove_dir_all(path).expect("Failed to clean up test directory");
    }

    #[test]
    #[cfg(all(feature = "getrandom"))]
    fn signature() {
        let keystore = setup_mem_test_keystore();
        let public = keystore
            .ecdsa_generate_new(None)
            .expect("Should generate key");
        let msg = b"test message";
        let sig = keystore
            .ecdsa_sign(&public, msg)
            .expect("Should sign")
            .expect("Should have signature");

        // Convert public key to k256::ecdsa::VerifyingKey
        let verifying_key =
            k256::ecdsa::VerifyingKey::from_sec1_bytes(public.to_sec1_bytes().as_ref())
                .expect("Should create verifying key");

        // Verify with correct message
        assert!(
            verifying_key.verify(msg, &sig).is_ok(),
            "Signature verification failed"
        );

        // Try to verify with wrong message
        let wrong_msg = b"wrong message";
        assert!(
            verifying_key.verify(wrong_msg, &sig).is_err(),
            "Signature should not verify with wrong message"
        );

        // Try to verify with wrong public key
        let wrong_public = keystore
            .ecdsa_generate_new(None)
            .expect("Should generate key");
        let wrong_verifying_key =
            k256::ecdsa::VerifyingKey::from_sec1_bytes(wrong_public.to_sec1_bytes().as_ref())
                .expect("Should create wrong verifying key");
        assert!(
            wrong_verifying_key.verify(msg, &sig).is_err(),
            "Signature should not verify with wrong public key"
        );
    }
}

mod bls381 {
    use super::*;

    #[test]
    #[cfg(all(feature = "getrandom", feature = "std"))]
    fn keys() {
        // In-Memory
        let mem_keystore = setup_mem_test_keystore();
        let mem_public = mem_keystore
            .bls381_generate_new(None)
            .expect("Should generate key");
        // Read back the key from in-memory storage
        let mem_read = mem_keystore.iter_bls381().next().unwrap();
        assert_eq!(
            mem_public.0, mem_read.0,
            "In-memory key should not be mutated in storage"
        );

        // Filesystem
        let (fs_keystore, path) = setup_fs_test_keystore("bls381");
        let fs_public = fs_keystore
            .bls381_generate_new(None)
            .expect("Should generate key");
        // Read back the key from filesystem storage
        let fs_read = fs_keystore.iter_bls381().next().unwrap();
        assert_eq!(
            fs_public.0, fs_read.0,
            "Filesystem key should not be mutated in storage"
        );

        // Clean up filesystem keystore
        std::fs::remove_dir_all(path).expect("Failed to clean up test directory");
    }

    #[test]
    #[cfg(all(feature = "getrandom"))]
    fn signature() {
        let keystore = setup_mem_test_keystore();
        let public = keystore
            .bls381_generate_new(None)
            .expect("Should generate key");
        let msg = b"test message";
        let sig = keystore
            .bls381_sign(&public, msg)
            .expect("Should sign")
            .expect("Should have signature");

        // Verify with correct message
        assert!(
            sig.verify(&w3f_bls::Message::from(msg.as_slice()), &public),
            "Signature verification failed"
        );

        // Try to verify with wrong message
        let wrong_msg = b"wrong message";
        assert!(
            !sig.verify(&w3f_bls::Message::from(wrong_msg.as_slice()), &public),
            "Signature should not verify with wrong message"
        );

        // Try to verify with wrong public key
        let wrong_public = keystore
            .bls381_generate_new(None)
            .expect("Should generate key");
        assert!(
            !sig.verify(&w3f_bls::Message::from(msg.as_slice()), &wrong_public),
            "Signature should not verify with wrong public key"
        );
    }
}

mod bn254 {
    use super::*;
    use crate::keystore::BackendExt;
    use eigensdk::crypto_bn254::utils::verify_message;

    #[test]
    #[cfg(all(feature = "getrandom", feature = "std"))]
    fn keys() {
        // In-Memory
        let mem_keystore = setup_mem_test_keystore();
        let mem_public = mem_keystore
            .bls_bn254_generate_new(None)
            .expect("Should generate key");
        // Read back the key from in-memory storage
        let mem_read = mem_keystore.bls_bn254_key().expect("Should read keys");
        assert_eq!(
            mem_public,
            mem_read.public_key(),
            "In-memory key should not be mutated in storage"
        );

        // Filesystem
        let (fs_keystore, path) = setup_fs_test_keystore("bn254");
        let fs_public = fs_keystore
            .bls_bn254_generate_new(None)
            .expect("Should generate key");
        // Read back the key from filesystem storage
        let fs_read = fs_keystore.bls_bn254_key().expect("Should read keys");
        assert_eq!(
            fs_public,
            fs_read.public_key(),
            "Filesystem key should not be mutated in storage"
        );

        // Clean up filesystem keystore
        std::fs::remove_dir_all(path).expect("Failed to clean up test directory");
    }

    #[test]
    #[cfg(all(feature = "getrandom"))]
    fn signature() {
        let keystore = setup_mem_test_keystore();
        let public = keystore
            .bls_bn254_generate_new(None)
            .expect("Should generate key");
        let msg: &[u8; 32] = b"testing messages testing message";
        let sig = keystore
            .bls_bn254_sign(&public, msg)
            .expect("Should sign")
            .expect("Should have signature");
        let pair = keystore.bls_bn254_key().unwrap();

        assert_eq!(public.g1(), pair.public_key().g1());

        // Verify with correct message
        assert!(
            verify_message(pair.public_key_g2().g2(), msg, sig),
            "Signature verification failed"
        );

        // Try to verify with wrong message
        let wrong_msg = b"wrong message";
        assert!(
            !verify_message(pair.public_key_g2().g2(), wrong_msg, sig),
            "Signature should not verify with wrong message"
        );
    }
}
