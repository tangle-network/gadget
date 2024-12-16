#![cfg_attr(not(feature = "std"), no_std)]

mod arkworks_bn254;
pub mod error;

use ark_bn254::{Bn254, Fq, Fr, G1Affine, G1Projective, G2Affine};
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};
use ark_ff::{BigInteger256, Field, One, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
pub use arkworks_bn254::*;
use error::{Bn254Error, Result};
use gadget_std::vec::Vec;
use num_bigint::BigUint;
use sha2::{Digest, Sha256};

/// Serialize this to a vector of bytes.
pub fn to_bytes<T: CanonicalSerialize>(elt: T) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(elt.compressed_size());

    <T as CanonicalSerialize>::serialize_compressed(&elt, &mut bytes).unwrap();

    bytes
}

/// Deserialize this from a slice of bytes.
pub fn from_bytes<T: CanonicalDeserialize>(bytes: &[u8]) -> T {
    <T as CanonicalDeserialize>::deserialize_compressed(&mut &bytes[..]).unwrap()
}

fn hash_to_curve(digest: &[u8]) -> G1Affine {
    let one = Fq::one();
    let three = Fq::from(3u64);

    let mut hasher = Sha256::new();
    hasher.update(digest);
    let hashed_result = hasher.finalize();

    // Convert digest to a big integer and then to a field element
    let mut x = {
        let big_int = BigUint::from_bytes_be(&hashed_result);
        let mut bytes = [0u8; 32];
        big_int
            .to_bytes_be()
            .iter()
            .rev()
            .enumerate()
            .for_each(|(i, &b)| bytes[i] = b);
        Fq::from_le_bytes_mod_order(&bytes)
    };

    loop {
        // y = x^3 + 3
        let mut y = x;
        y.square_in_place();
        y *= x;
        y += three;

        // Check if y is a quadratic residue (i.e., has a square root in the field)
        if let Some(y) = y.sqrt() {
            return G1Projective::new(x, y, Fq::one()).into_affine();
        } else {
            // x = x + 1
            x += one;
        }
    }
}

pub fn sign(sk: Fr, message: &[u8]) -> Result<G1Affine> {
    let q = hash_to_curve(message);

    let sk_int: BigInteger256 = sk.into();
    let r = q.mul_bigint(sk_int);

    if !r.into_affine().is_on_curve() || !r.into_affine().is_in_correct_subgroup_assuming_on_curve()
    {
        return Err(Bn254Error::SignatureNotInSubgroup);
    }

    Ok(r.into_affine())
}

pub fn verify(public_key: G2Affine, message: &[u8], signature: G1Affine) -> bool {
    if !signature.is_in_correct_subgroup_assuming_on_curve() || !signature.is_on_curve() {
        return false;
    }

    let q = hash_to_curve(message);
    let c1 = Bn254::pairing(q, public_key);
    let c2 = Bn254::pairing(signature, G2Affine::generator());
    c1 == c2
}
