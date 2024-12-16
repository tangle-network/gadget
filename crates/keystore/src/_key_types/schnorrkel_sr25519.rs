use crate::_key_types::KeyType;
use crate::error::{Error, Result};
use gadget_std::UniformRand;

/// Schnorrkel key type
pub struct SchnorrkelSr25519;

macro_rules! impl_schnorrkel_serde {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, PartialEq, Eq, Debug)]
        pub struct $name(pub $inner);

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                self.0.to_bytes().partial_cmp(&other.0.to_bytes())
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.0.to_bytes().cmp(&other.0.to_bytes())
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> core::result::Result<S::Ok, S::Error> {
                serializer.serialize_bytes(&self.0.to_bytes())
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

impl_schnorrkel_serde!(Public, schnorrkel::PublicKey);
impl_schnorrkel_serde!(Secret, schnorrkel::SecretKey);
impl_schnorrkel_serde!(SchnorrkelSignature, schnorrkel::Signature);

impl KeyType for SchnorrkelSr25519 {
    type Public = Public;
    type Secret = Secret;
    type Signature = SchnorrkelSignature;

    fn key_type_id() -> super::KeyTypeId {
        super::KeyTypeId::SchnorrkelSr25519
    }

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret> {
        let secret_key = if let Some(seed) = seed {
            schnorrkel::SecretKey::from_bytes(seed).map_err(|e| Error::InvalidSeed(e.to_string()))
        } else {
            let mut rng = Self::get_rng();
            let rand_bytes: [u8; 32] = <[u8; 32]>::rand(&mut rng);
            schnorrkel::SecretKey::from_bytes(&rand_bytes)
                .map_err(|e| Error::InvalidSeed(e.to_string()))
        };

        secret_key.map(Secret)
    }

    fn generate_with_string(secret: String) -> Result<Self::Secret> {
        let hex_encoded = hex::decode(secret).map_err(|_| Error::InvalidHexDecoding)?;
        let secret_key = schnorrkel::SecretKey::from_bytes(&hex_encoded)
            .map_err(|e| Error::InvalidSeed(e.to_string()))?;
        Ok(Secret(secret_key))
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        Public(secret.0.to_public())
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
        if public.0.verify(ctx, &signature.0).is_ok() {
            true
        } else {
            false
        }
    }
}
