//! BLS BN254 keys and signatures

use crate::keystore::Error;
use alloy_primitives::keccak256;
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{PrimeField, UniformRand};
use ark_serialize::CanonicalDeserialize;
use eigensdk::crypto_bls::PublicKey;
use eigensdk::crypto_bls::{BlsG1Point, BlsSignature, PrivateKey};
use eigensdk::crypto_bn254::utils::map_to_curve;
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

#[must_use]
pub fn generate_with_optional_seed(seed: Option<&[u8]>) -> Secret {
    match seed {
        Some(s) => {
            let hashed_seed = keccak256(s);
            Secret::from_le_bytes_mod_order(hashed_seed.as_slice())
        }
        None => {
            let mut rng = rand::thread_rng();
            Secret::rand(&mut rng)
        }
    }
}

pub fn sign(secret: &mut Secret, msg: &[u8; 32]) -> Signature {
    let hashed_point = map_to_curve(msg);
    let sig = hashed_point.mul_bigint(secret.0);
    Signature::from(sig)
}

#[must_use]
pub fn to_public(secret: &Secret) -> Public {
    let public = PublicKey::generator().mul_bigint(secret.0);
    Public::new(public.into_affine())
}

pub fn secret_from_bytes(bytes: &[u8]) -> Result<Secret, Error> {
    Secret::deserialize_uncompressed(bytes).map_err(|e| Error::BlsBn254(e.to_string()))
}
