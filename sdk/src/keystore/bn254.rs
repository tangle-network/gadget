//! BLS BN254 keys and signatures

use crate::keystore::Error;
use alloy_primitives::keccak256;
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{PrimeField, UniformRand};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use eigensdk::crypto_bls::{BlsG1Point, BlsSignature, PrivateKey};
use eigensdk::crypto_bls::{BlsKeyPair, PublicKey};
use elliptic_curve::rand_core::OsRng;
use k256::sha2::{Digest, Sha256};

/// BN254 public key.
pub type Public = BlsG1Point;
/// BN254 secret key.
pub type Secret = PrivateKey;
/// BN254 signature.
pub type Signature = BlsSignature;

/// Creates a hash of the public key
///
/// Used for BLS BN254 keys in the keystore, as the public keys are too long to be used as file names.
/// Instead of storing public keys in the filename, the public key is stored in `path/to/key/<hash>.pub` while the private
/// key is stored in `path/to/key/<hash>`
pub fn hash_public(public: Public) -> Result<String, Error> {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&public).map_err(|e| Error::BlsBn254(e.to_string()))?);
    let hashed_public = hasher.finalize();
    Ok(hex::encode(hashed_public))
}

pub fn generate_with_optional_seed(seed: Option<&[u8]>) -> Result<Secret, Error> {
    match seed {
        Some(s) => {
            let hashed_seed = keccak256(s);
            Ok(Secret::from_le_bytes_mod_order(hashed_seed.as_slice()))
        }
        None => {
            let mut buffer = Vec::new();
            let private_key = Secret::rand(&mut OsRng);
            private_key
                .serialize_uncompressed(&mut buffer)
                .map_err(|_| {
                    Error::BlsBn254("Secret serialization failed during generation".to_string())
                })?;
            let secret = hex::encode(buffer);
            Ok(Secret::from_le_bytes_mod_order(secret.as_bytes()))
        }
    }
}

pub fn sign(secret: &mut Secret, msg: &[u8; 32]) -> Signature {
    let pair = BlsKeyPair::new(secret.to_string()).unwrap();
    pair.sign_message(msg.as_slice()).g1_point().g1()
}

#[must_use]
pub fn to_public(secret: &Secret) -> Public {
    let public = PublicKey::generator().mul_bigint(secret.0);
    Public::new(public.into_affine())
}

pub fn secret_from_bytes(bytes: &[u8]) -> Result<Secret, Error> {
    Secret::deserialize_uncompressed(bytes).map_err(|e| Error::BlsBn254(e.to_string()))
}
