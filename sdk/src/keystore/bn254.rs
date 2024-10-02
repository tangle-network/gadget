//! BLS BN254 keys and signatures

use alloy_primitives::keccak256;
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{PrimeField, UniformRand};
use eigensdk::crypto_bls::PublicKey;
use eigensdk::crypto_bls::{BlsG1Point, BlsSignature, PrivateKey};
use eigensdk::crypto_bn254::utils::map_to_curve;

/// BN254 public key.
pub type Public = BlsG1Point;
/// BN254 secret key.
pub type Secret = PrivateKey;
/// BN254 signature.
pub type Signature = BlsSignature;

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

#[must_use]
pub fn secret_from_bytes(bytes: &[u8]) -> Secret {
    Secret::from_le_bytes_mod_order(bytes)
}
