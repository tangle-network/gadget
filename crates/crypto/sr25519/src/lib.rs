#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;
use error::{Result, Sr25519Error};

#[cfg(test)]
mod tests;

use gadget_crypto_core::BytesEncoding;
use gadget_crypto_core::{KeyType, KeyTypeId};
use gadget_std::{
    hash::Hash,
    string::{String, ToString},
    vec::Vec,
};
use schnorrkel::MiniSecretKey;

/// Schnorrkel key type
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct SchnorrkelSr25519;

macro_rules! impl_schnorrkel_serde {
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
                self.0.to_bytes().cmp(&other.0.to_bytes())
            }
        }

        impl Hash for $name {
            fn hash<H: gadget_std::hash::Hasher>(&self, state: &mut H) {
                self.0.to_bytes().hash(state);
            }
        }

        impl BytesEncoding for $name {
            fn to_bytes(&self) -> Vec<u8> {
                self.0.to_bytes().to_vec()
            }

            fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, serde::de::value::Error> {
                <$inner>::from_bytes(bytes)
                    .map(Self)
                    .map_err(|e| serde::de::Error::custom(e.to_string()))
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> core::result::Result<S::Ok, S::Error> {
                <Vec<u8>>::serialize(&self.to_bytes(), serializer)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(
                deserializer: D,
            ) -> core::result::Result<Self, D::Error> {
                let bytes = <Vec<u8>>::deserialize(deserializer)?;
                let inner = <$inner>::from_bytes(&bytes)
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?;
                Ok($name(inner))
            }
        }
    };
}

impl_schnorrkel_serde!(SchnorrkelPublic, schnorrkel::PublicKey);
impl_schnorrkel_serde!(SchnorrkelSecret, schnorrkel::SecretKey);
impl_schnorrkel_serde!(SchnorrkelSignature, schnorrkel::Signature);

impl KeyType for SchnorrkelSr25519 {
    type Secret = SchnorrkelSecret;
    type Public = SchnorrkelPublic;
    type Signature = SchnorrkelSignature;
    type Error = Sr25519Error;

    fn key_type_id() -> KeyTypeId {
        KeyTypeId::Sr25519
    }

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret> {
        let secret_key = if let Some(seed) = seed {
            // Pad seed to 64 bytes or error if too large
            if seed.len() > 64 {
                return Err(Sr25519Error::InvalidSeed(
                    "Seed must not exceed 64 bytes".into(),
                ));
            }
            let mut padded_seed = [0u8; 64];
            padded_seed[..seed.len()].copy_from_slice(seed);
            schnorrkel::SecretKey::from_bytes(&padded_seed)
                .map_err(|e| Sr25519Error::InvalidSeed(e.to_string()))
        } else {
            let mut rng = Self::get_rng();
            let mini_secret_key = MiniSecretKey::generate_with(&mut rng);
            Ok(mini_secret_key.expand(MiniSecretKey::UNIFORM_MODE))
        };

        secret_key.map(SchnorrkelSecret)
    }

    fn generate_with_string(secret: String) -> Result<Self::Secret> {
        let hex_encoded = hex::decode(secret)?;
        let secret_key = schnorrkel::SecretKey::from_bytes(&hex_encoded)
            .map_err(|e| Sr25519Error::InvalidSeed(e.to_string()))?;
        Ok(SchnorrkelSecret(secret_key))
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        SchnorrkelPublic(secret.0.to_public())
    }

    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature> {
        let ctx = schnorrkel::signing_context(b"tangle").bytes(msg);
        Ok(SchnorrkelSignature(
            secret.0.sign(ctx, &secret.0.to_public()),
        ))
    }

    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature> {
        let ctx = schnorrkel::signing_context(b"tangle").bytes(msg);
        Ok(SchnorrkelSignature(
            secret.0.sign(ctx, &secret.0.to_public()),
        ))
    }

    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
        let ctx = schnorrkel::signing_context(b"tangle").bytes(msg);
        public.0.verify(ctx, &signature.0).is_ok()
    }
}
