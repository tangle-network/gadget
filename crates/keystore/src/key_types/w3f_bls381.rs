use crate::error::{Error, Result};
use crate::key_types::{from_bytes, to_bytes, KeyType};
use gadget_std::UniformRand;
use w3f_bls::{Message, PublicKey, SecretKey, SerializableToBytes, Signature, TinyBLS381};
/// BLS381 key type
pub struct W3fBls381;

pub const CONTEXT: &[u8] = b"tangle";

macro_rules! impl_w3f_serde {
    ($name:ident, $inner:ty) => {
        #[derive(Clone)]
        pub struct $name(pub $inner);

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                to_bytes(self.0.clone()) == to_bytes(other.0.clone())
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                to_bytes(self.0.clone()).partial_cmp(&to_bytes(other.0.clone()))
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                to_bytes(self.0.clone()).cmp(&to_bytes(other.0.clone()))
            }
        }

        impl gadget_std::fmt::Debug for $name {
            fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                write!(f, "{:?}", to_bytes(self.0.clone()))
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> core::result::Result<S::Ok, S::Error> {
                serializer.serialize_bytes(&to_bytes(self.0.clone()))
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                // Deserialize as bytes
                let bytes = <Vec<u8>>::deserialize(deserializer)?;

                // Convert bytes back to inner type
                let inner = from_bytes::<$inner>(&bytes);

                Ok($name(inner))
            }
        }
    };
}

impl_w3f_serde!(Public, PublicKey<TinyBLS381>);
impl_w3f_serde!(Secret, SecretKey<TinyBLS381>);
impl_w3f_serde!(W3fBls381Signature, Signature<TinyBLS381>);

impl KeyType for W3fBls381 {
    type Public = Public;
    type Secret = Secret;
    type Signature = W3fBls381Signature;

    fn key_type_id() -> super::KeyTypeId {
        super::KeyTypeId::W3fBls381
    }

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret> {
        if let Some(seed) = seed {
            Ok(Secret(SecretKey::from_seed(seed)))
        } else {
            // Should only be used for testing. Pass a seed in production.
            let mut rng = gadget_std::test_rng();
            let rand_bytes = <[u8; 32]>::rand(&mut rng);
            Ok(Secret(SecretKey::from_seed(&rand_bytes)))
        }
    }

    fn generate_with_string(secret: String) -> Result<Self::Secret> {
        let hex_encoded = hex::decode(secret).map_err(|_| Error::InvalidHexDecoding)?;
        let secret =
            SecretKey::from_bytes(&hex_encoded).map_err(|e| Error::InvalidSeed(e.to_string()))?;
        Ok(Secret(secret))
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        Public(secret.0.into_public())
    }

    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature> {
        let mut rng = Self::get_rng();
        let message: Message = Message::new(CONTEXT, msg);
        Ok(W3fBls381Signature(secret.0.sign(&message, &mut rng)))
    }

    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature> {
        let mut rng = Self::get_rng();
        let message: Message = Message::new(CONTEXT, msg);
        Ok(W3fBls381Signature(secret.0.sign(&message, &mut rng)))
    }

    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
        let message = Message::new(CONTEXT, msg);
        signature.0.verify(&message, &public.0)
    }
}
