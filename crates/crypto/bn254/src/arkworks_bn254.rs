use crate::error::{Bn254Error, Result};
use ark_bn254::{Fr, G1Affine, G2Affine};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{PrimeField, UniformRand};
use gadget_crypto_core::{KeyType, KeyTypeId};
use gadget_std::{
    format,
    str::FromStr,
    string::{String, ToString},
    vec::Vec,
};

use super::{from_bytes, sign, to_bytes, verify};

/// BLS-BN254 key type
pub struct ArkBlsBn254;

macro_rules! impl_ark_serde {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, PartialEq, Eq, Debug)]
        pub struct $name(pub $inner);

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                to_bytes(self.0).partial_cmp(&to_bytes(other.0))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                to_bytes(self.0).cmp(&to_bytes(other.0))
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> core::result::Result<S::Ok, S::Error> {
                serializer.serialize_bytes(&to_bytes(self.0))
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
        KeyTypeId::ArkBn254
    }

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret> {
        let secret = if let Some(seed) = seed {
            let seed = gadget_std::str::from_utf8(seed)
                .map_err(|e| Bn254Error::InvalidSeed(e.to_string()))?;
            Fr::from_str(seed).map_err(|e| Bn254Error::InvalidSeed(format!("{:?}", e)))?
        } else {
            let mut rng = Self::get_rng();
            Fr::rand(&mut rng)
        };
        Ok(ArkBlsBn254Secret(secret))
    }

    fn generate_with_string(secret: String) -> Result<Self::Secret> {
        let secret =
            Fr::from_str(&secret).map_err(|e| Bn254Error::InvalidSeed(format!("{:?}", e)))?;
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

// Continue with KeyType implementation...
