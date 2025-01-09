#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
use error::{Bn254Error, Result};

#[cfg(test)]
mod tests;

use ark_bn254::{Bn254, Fq, Fr, G1Affine, G1Projective, G2Affine};
use ark_ec::{pairing::Pairing, AffineRepr, CurveGroup};
use ark_ff::UniformRand;
use ark_ff::{BigInteger256, Field, One, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use gadget_crypto_core::KeyEncoding;
use gadget_crypto_core::{KeyType, KeyTypeId};
use gadget_std::vec::Vec;
use gadget_std::{
    str::FromStr,
    string::{String, ToString},
};
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

/// BLS-BN254 key type
pub struct ArkBlsBn254;

macro_rules! impl_ark_serde {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, PartialEq, Eq, Debug)]
        pub struct $name(pub $inner);

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                self.to_bytes().cmp(&other.to_bytes())
            }
        }

        impl KeyEncoding for $name {
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
            let mut rng = Self::get_rng();
            Fr::rand(&mut rng)
        };
        Ok(ArkBlsBn254Secret(secret))
    }

    fn generate_with_string(secret: String) -> Result<Self::Secret> {
        let secret = Fr::from_str(&secret)
            .map_err(|_| Bn254Error::InvalidSeed("Invalid secret string".to_string()))?;
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
