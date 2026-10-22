#![cfg_attr(not(feature = "std"), no_std)]

pub mod aggregation;
pub mod error;
use error::{Bn254Error, Result};

#[cfg(test)]
mod tests;

use ark_bn254::{Bn254, Fq, Fr, G1Affine, G1Projective, G2Affine};
use ark_ec::{AffineRepr, CurveGroup, pairing::Pairing};
#[cfg(feature = "std")]
use ark_ff::UniformRand;
use ark_ff::{BigInteger256, Field, One, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use blueprint_crypto_core::BytesEncoding;
use blueprint_crypto_core::{KeyType, KeyTypeId};
use blueprint_std::hash::Hash;
use blueprint_std::vec::Vec;
use blueprint_std::{
    str::FromStr,
    string::{String, ToString},
};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sha3::Keccak256;

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
        }
        // x = x + 1
        x += one;
    }
}

/// Map a Tangle contract message to G1 with the TNT core algorithm.
///
/// TNT core uses Keccak-256, then tries at most 256 consecutive x values.
/// Its modular square root selects a specific y coordinate, so Arkworks'
/// canonical `sqrt` result is not interchangeable here.
fn hash_to_curve_tangle(message: &[u8]) -> Result<G1Affine> {
    let one = Fq::one();
    let three = Fq::from(3u64);
    let digest = Keccak256::digest(message);
    let mut x = Fq::from_be_bytes_mod_order(&digest);

    // (P_MOD + 1) / 4 in little-endian 64-bit limbs.
    const SQRT_EXPONENT: [u64; 4] = [
        0x4f08_2305_b61f_3f52,
        0x65e0_5aa4_5a1c_72a3,
        0x6e14_116d_a060_5617,
        0x0c19_139c_b84c_680a,
    ];

    for _ in 0..256 {
        let mut y_squared = x;
        y_squared.square_in_place();
        y_squared *= x;
        y_squared += three;

        let y = y_squared.pow(SQRT_EXPONENT);
        if y * y == y_squared {
            return Ok(G1Projective::new(x, y, Fq::one()).into_affine());
        }
        x += one;
    }

    Err(Bn254Error::HashToPointFailed)
}

/// Sign the exact byte message verified by TNT core's BN254 library.
pub fn sign_tangle(sk: Fr, message: &[u8]) -> Result<G1Affine> {
    let point = hash_to_curve_tangle(message)?;
    let signature = point.mul_bigint(BigInteger256::from(sk)).into_affine();

    if signature.is_zero() {
        return Err(Bn254Error::DegeneratePoint);
    }
    if !signature.is_on_curve() || !signature.is_in_correct_subgroup_assuming_on_curve() {
        return Err(Bn254Error::SignatureNotInSubgroup);
    }

    Ok(signature)
}

/// Verify a signature with the exact TNT core BN254 hash-to-curve algorithm.
pub fn verify_tangle(public_key: G2Affine, message: &[u8], signature: G1Affine) -> bool {
    if signature.is_zero() || public_key.is_zero() {
        return false;
    }
    if !signature.is_on_curve()
        || !signature.is_in_correct_subgroup_assuming_on_curve()
        || !public_key.is_on_curve()
        || !public_key.is_in_correct_subgroup_assuming_on_curve()
    {
        return false;
    }

    let Ok(point) = hash_to_curve_tangle(message) else {
        return false;
    };
    Bn254::pairing(point, public_key) == Bn254::pairing(signature, G2Affine::generator())
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

/// BLS-BN254 key type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArkBlsBn254;

macro_rules! impl_ark_serde {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, PartialEq, Eq, Debug)]
        pub struct $name(pub $inner);

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<blueprint_std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> blueprint_std::cmp::Ordering {
                self.to_bytes().cmp(&other.to_bytes())
            }
        }

        impl Hash for $name {
            fn hash<H: blueprint_std::hash::Hasher>(&self, state: &mut H) {
                self.to_bytes().hash(state);
            }
        }

        impl BytesEncoding for $name {
            fn to_bytes(&self) -> Vec<u8> {
                crate::to_bytes(self.0)
            }

            fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, serde::de::value::Error> {
                let inner = from_bytes::<$inner>(&bytes);
                Ok($name(inner))
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> core::result::Result<S::Ok, S::Error> {
                let bytes = self.to_bytes();
                Vec::serialize(&bytes, serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let bytes = <Vec<u8>>::deserialize(deserializer)?;
                let inner = from_bytes::<$inner>(&bytes);
                Ok($name(inner))
            }
        }
    };
}

impl_ark_serde!(ArkBlsBn254Public, G2Affine);
impl_ark_serde!(ArkBlsBn254Secret, Fr);
impl_ark_serde!(ArkBlsBn254Signature, G1Affine);

impl ArkBlsBn254 {
    /// Sign a message for TNT core contracts without changing the key format.
    pub fn sign_tangle_with_secret(
        secret: &ArkBlsBn254Secret,
        message: &[u8],
    ) -> Result<ArkBlsBn254Signature> {
        sign_tangle(secret.0, message).map(ArkBlsBn254Signature)
    }

    /// Verify a signature with TNT core's contract-compatible algorithm.
    pub fn verify_tangle(
        public_key: &ArkBlsBn254Public,
        message: &[u8],
        signature: &ArkBlsBn254Signature,
    ) -> bool {
        verify_tangle(public_key.0, message, signature.0)
    }
}

impl zeroize::Zeroize for ArkBlsBn254Secret {
    fn zeroize(&mut self) {
        let ptr = (&raw mut self.0).cast::<u8>();
        let len = core::mem::size_of::<Fr>();
        unsafe { core::ptr::write_bytes(ptr, 0, len) };
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}

impl Drop for ArkBlsBn254Secret {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.zeroize();
    }
}

impl KeyType for ArkBlsBn254 {
    type Public = ArkBlsBn254Public;
    type Secret = ArkBlsBn254Secret;
    type Signature = ArkBlsBn254Signature;
    type Error = Bn254Error;

    fn key_type_id() -> KeyTypeId {
        KeyTypeId::Bn254
    }

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret> {
        let secret = if let Some(seed) = seed {
            Fr::from_random_bytes(seed)
                .ok_or_else(|| Bn254Error::InvalidSeed("None value".to_string()))?
        } else {
            #[cfg(feature = "std")]
            {
                let mut rng = blueprint_std::RandCore06::new();
                Fr::rand(&mut rng)
            }
            #[cfg(not(feature = "std"))]
            return Err(Bn254Error::InvalidSeed(
                "Random key generation requires the std feature".into(),
            ));
        };
        Ok(ArkBlsBn254Secret(secret))
    }

    fn generate_with_string(secret: String) -> Result<Self::Secret> {
        let secret = Fr::from_str(&secret)
            .map_err(|()| Bn254Error::InvalidSeed("Invalid secret string".to_string()))?;
        Ok(ArkBlsBn254Secret(secret))
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        ArkBlsBn254Public(
            G2Affine::generator()
                .mul_bigint(secret.0.into_bigint())
                .into_affine(),
        )
    }

    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature> {
        let signature =
            sign(secret.0, msg).map_err(|e| Bn254Error::SignatureFailed(e.to_string()))?;
        Ok(ArkBlsBn254Signature(signature))
    }

    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature> {
        let signature =
            sign(secret.0, msg).map_err(|e| Bn254Error::SignatureFailed(e.to_string()))?;
        Ok(ArkBlsBn254Signature(signature))
    }

    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
        verify(public.0, msg, signature.0)
    }
}
