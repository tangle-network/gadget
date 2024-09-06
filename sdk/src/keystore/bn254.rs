//! BLS BN254 keys and signatures

use alloy_primitives::keccak256;
use ark_ec::Group;
use ark_ff::{PrimeField, UniformRand};
use eigensdk_rs::eigen_utils::crypto::bls::{self, g1_projective_to_g1_point};
use eigensdk_rs::eigen_utils::crypto::bn254::{map_to_curve, mul_by_generator_g1};

/// BN254 public key.
pub type Public = bls::G1Point;
/// BN254 secret key.
pub type Secret = bls::PrivateKey;
/// BN254 signature.
pub type Signature = bls::Signature;

#[must_use]
pub fn generate_with_optional_seed(seed: Option<&[u8]>) -> Secret {
    match seed {
        Some(s) => {
            let hashed_seed = keccak256(s);
            Secret::from_le_bytes_mod_order(&hashed_seed.as_slice())
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
    Signature {
        g1_point: g1_projective_to_g1_point(&sig),
    }
}

#[must_use]
pub fn to_public(secret: &Secret) -> Public {
    let public = mul_by_generator_g1(secret.clone());
    g1_projective_to_g1_point(&public)
}

#[must_use]
pub fn secret_from_bytes(bytes: &[u8]) -> Secret {
    Secret::from_le_bytes_mod_order(bytes)
}
