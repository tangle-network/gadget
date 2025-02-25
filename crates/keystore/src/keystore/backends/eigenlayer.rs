use super::*;
use crate::{Error, Keystore};
use bn254::Bn254Backend;
use gadget_crypto::k256::K256Ecdsa;
use gadget_crypto::k256::{K256Signature, K256SigningKey, K256VerifyingKey};
use gadget_crypto::{BytesEncoding, KeyTypeId};

#[async_trait::async_trait]
pub trait EigenlayerBackend: Bn254Backend {
    /// Generate a new ECDSA key pair from seed
    fn ecdsa_generate_new(&self, seed: Option<&[u8]>) -> Result<K256VerifyingKey>;

    /// Generate an ECDSA key pair from a string seed
    fn ecdsa_generate_from_string(&self, secret: &str) -> Result<K256VerifyingKey>;

    /// Sign a message using ECDSA key
    fn ecdsa_sign(&self, public: &K256VerifyingKey, msg: &[u8]) -> Result<K256Signature>;

    /// Get the secret key for an ECDSA public key
    fn expose_ecdsa_secret(&self, public: &K256VerifyingKey) -> Result<Option<K256SigningKey>>;

    /// Iterate over all ECDSA public keys
    fn iter_ecdsa(&self) -> Box<dyn Iterator<Item = K256VerifyingKey> + '_>;
}

#[async_trait::async_trait]
impl EigenlayerBackend for Keystore {
    fn ecdsa_generate_new(&self, seed: Option<&[u8]>) -> Result<K256VerifyingKey> {
        let secret =
            K256Ecdsa::generate_with_seed(seed).map_err(|e| Error::Other(e.to_string()))?;
        insert_ecdsa_key(self, &secret)
    }

    fn ecdsa_generate_from_string(&self, seed_string: &str) -> Result<K256VerifyingKey> {
        let secret = K256Ecdsa::generate_with_string(seed_string.to_string())
            .map_err(|e| Error::Other(e.to_string()))?;
        insert_ecdsa_key(self, &secret)
    }

    fn ecdsa_sign(&self, public: &K256VerifyingKey, msg: &[u8]) -> Result<K256Signature> {
        if let Some(mut secret) = self.expose_ecdsa_secret(public)? {
            Ok(K256Ecdsa::sign_with_secret(&mut secret, msg)
                .map_err(|e| Error::Other(e.to_string()))?)
        } else {
            Err(Error::SignatureFailed("ECDSA key not found".into()))
        }
    }

    fn expose_ecdsa_secret(&self, public: &K256VerifyingKey) -> Result<Option<K256SigningKey>> {
        let public_bytes = public.to_bytes();

        if let Some(storages) = self.storages.get(&KeyTypeId::Ecdsa) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_secret_raw(KeyTypeId::Ecdsa, public_bytes.clone())?
                {
                    return Ok(Some(K256SigningKey::from_bytes(&secret_bytes)?));
                }
            }
        }

        Ok(None)
    }

    fn iter_ecdsa(&self) -> Box<dyn Iterator<Item = K256VerifyingKey> + '_> {
        let Some(storages) = self.storages.get(&KeyTypeId::Ecdsa) else {
            return Box::new(std::iter::empty());
        };

        let mut keys = Vec::new();
        for entry in storages {
            let mut storage_keys = entry
                .storage
                .list_raw(KeyTypeId::Ecdsa)
                .filter_map(|bytes| K256VerifyingKey::from_bytes(&bytes).ok())
                .collect::<Vec<_>>();
            keys.append(&mut storage_keys);
        }
        Box::new(keys.into_iter())
    }
}

fn insert_ecdsa_key(keystore: &Keystore, secret: &K256SigningKey) -> Result<K256VerifyingKey> {
    let public = K256Ecdsa::public_from_secret(secret);
    let public_bytes = public.to_bytes();
    let secret_bytes = secret.to_bytes();

    if let Some(storages) = keystore.storages.get(&KeyTypeId::Ecdsa) {
        for entry in storages {
            entry.storage.store_raw(
                KeyTypeId::Ecdsa,
                public_bytes.clone(),
                secret_bytes.clone(),
            )?;
        }
    }

    Ok(public)
}
