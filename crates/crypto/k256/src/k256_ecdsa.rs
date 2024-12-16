use gadget_crypto_core::{KeyType, KeyTypeId};
use gadget_std::UniformRand;
use gadget_std::{
    string::{String, ToString},
    vec::Vec,
};
use k256::ecdsa::signature::SignerMut;

use crate::error::{K256Error, Result};

/// ECDSA key type
pub struct K256Ecdsa;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct K256VerifyingKey(pub k256::ecdsa::VerifyingKey);

impl PartialOrd for K256VerifyingKey {
    fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
        self.0.to_sec1_bytes().partial_cmp(&other.0.to_sec1_bytes())
    }
}

impl Ord for K256VerifyingKey {
    fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
        self.0.to_sec1_bytes().cmp(&other.0.to_sec1_bytes())
    }
}

impl serde::Serialize for K256VerifyingKey {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = self.0.to_sec1_bytes();
        serializer.serialize_bytes(&bytes)
    }
}

impl<'de> serde::Deserialize<'de> for K256VerifyingKey {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = <Vec<u8>>::deserialize(deserializer)?;
        let verifying_key = k256::ecdsa::VerifyingKey::from_sec1_bytes(&bytes)
            .map_err(|e| serde::de::Error::custom(e.to_string()))?;
        Ok(K256VerifyingKey(verifying_key))
    }
}

macro_rules! impl_serde_bytes {
    ($wrapper:ident, $inner:path) => {
        #[derive(Clone, PartialEq, Eq, Debug)]
        pub struct $wrapper(pub $inner);

        impl PartialOrd for $wrapper {
            fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                self.0.to_bytes().partial_cmp(&other.0.to_bytes())
            }
        }

        impl Ord for $wrapper {
            fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                self.0.to_bytes().cmp(&other.0.to_bytes())
            }
        }

        impl serde::Serialize for $wrapper {
            fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let bytes = self.0.to_bytes();
                serializer.serialize_bytes(&bytes)
            }
        }

        impl<'de> serde::Deserialize<'de> for $wrapper {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let bytes = <Vec<u8>>::deserialize(deserializer)?;
                let inner = <$inner>::from_slice(&bytes)
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?;
                Ok($wrapper(inner))
            }
        }
    };
}

impl_serde_bytes!(K256SigningKey, k256::ecdsa::SigningKey);
impl_serde_bytes!(K256Signature, k256::ecdsa::Signature);

impl KeyType for K256Ecdsa {
    type Public = K256VerifyingKey;
    type Secret = K256SigningKey;
    type Signature = K256Signature;
    type Error = K256Error;

    fn key_type_id() -> KeyTypeId {
        KeyTypeId::K256Ecdsa
    }

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret> {
        let signing_key = if let Some(seed) = seed {
            k256::ecdsa::SigningKey::from_bytes(seed.into())
                .map_err(|e| K256Error::InvalidSeed(e.to_string()))
        } else {
            let mut rng = Self::get_rng();
            let rand_bytes: [u8; 32] = <[u8; 32]>::rand(&mut rng);
            k256::ecdsa::SigningKey::from_slice(&rand_bytes)
                .map_err(|e| K256Error::InvalidSeed(e.to_string()))
        };

        signing_key.map(K256SigningKey)
    }

    fn generate_with_string(secret: String) -> Result<Self::Secret> {
        let hex_encoded = hex::decode(secret)?;
        let signing_key = k256::ecdsa::SigningKey::from_slice(&hex_encoded)
            .map_err(|e| K256Error::InvalidSeed(e.to_string()))?;
        Ok(K256SigningKey(signing_key))
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        K256VerifyingKey(secret.0.verifying_key().clone())
    }

    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature> {
        let sig = secret.0.sign(msg);
        Ok(K256Signature(sig))
    }

    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature> {
        let (sig, _) = secret
            .0
            .sign_prehash_recoverable(msg)
            .map_err(|e| K256Error::SignatureFailed(e.to_string()))?;
        Ok(K256Signature(sig))
    }

    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
        use k256::ecdsa::signature::Verifier;
        public.0.verify(msg, &signature.0).is_ok()
    }
}
