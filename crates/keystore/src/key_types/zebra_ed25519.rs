use crate::error::{Error, Result};
use crate::key_types::KeyType;

/// Ed25519 key type
pub struct Ed25519Zebra;

macro_rules! impl_zebra_serde {
    ($name:ident, $inner:ty) => {
        #[derive(Clone)]
        pub struct $name(pub $inner);

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.0.as_ref() == other.0.as_ref()
            }
        }

        impl Eq for $name {}

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                self.0.as_ref().partial_cmp(other.0.as_ref())
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.0.as_ref().cmp(other.0.as_ref())
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:?}", self.0.as_ref())
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                // Get the raw bytes
                let bytes = self.0.as_ref();
                serializer.serialize_bytes(bytes)
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
                let inner = <$inner>::try_from(bytes.as_slice())
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?;

                Ok($name(inner))
            }
        }
    };
}

impl_zebra_serde!(Ed25519SigningKey, ed25519_zebra::SigningKey);
impl_zebra_serde!(Ed25519VerificationKey, ed25519_zebra::VerificationKey);
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Ed25519Signature(pub ed25519_zebra::Signature);

impl PartialOrd for Ed25519Signature {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.to_bytes().partial_cmp(&other.0.to_bytes())
    }
}

impl Ord for Ed25519Signature {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.to_bytes().cmp(&other.0.to_bytes())
    }
}

impl serde::Serialize for Ed25519Signature {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Get the raw bytes
        let bytes = self.0.to_bytes();
        serializer.serialize_bytes(&bytes)
    }
}

impl<'de> serde::Deserialize<'de> for Ed25519Signature {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize as bytes
        let bytes = <Vec<u8>>::deserialize(deserializer)?;

        // Convert bytes back to inner type
        let inner = ed25519_zebra::Signature::try_from(bytes.as_slice())
            .map_err(|e| serde::de::Error::custom(e.to_string()))?;

        Ok(Ed25519Signature(inner))
    }
}

impl KeyType for Ed25519Zebra {
    type Public = Ed25519VerificationKey;
    type Secret = Ed25519SigningKey;
    type Signature = Ed25519Signature;

    fn key_type_id() -> super::KeyTypeId {
        super::KeyTypeId::ZebraEd25519
    }

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret> {
        if let Some(seed) = seed {
            let seed = <[u8; 32]>::try_from(seed)
                .map_err(|_| Error::InvalidSeed("Seed is not 32 bytes!".to_string()))?;
            Ok(Ed25519SigningKey(ed25519_zebra::SigningKey::from(seed)))
        } else {
            let mut rng = Self::get_rng();
            Ok(Ed25519SigningKey(ed25519_zebra::SigningKey::new(&mut rng)))
        }
    }

    fn generate_with_string(secret: String) -> Result<Self::Secret> {
        let hex_encoded = hex::decode(secret).map_err(|_| Error::InvalidHexDecoding)?;
        let secret = ed25519_zebra::SigningKey::try_from(hex_encoded.as_slice())
            .map_err(|e| Error::InvalidSeed(e.to_string()))?;
        Ok(Ed25519SigningKey(secret))
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        Ed25519VerificationKey((&secret.0).into())
    }

    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature> {
        Ok(Ed25519Signature(secret.0.sign(msg)))
    }

    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature> {
        Ok(Ed25519Signature(secret.0.sign(msg)))
    }

    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
        public.0.verify(&signature.0, msg).is_ok()
    }
}
