use super::*;
use crate::error::{Error, Result};
use crate::Keystore;
use gadget_crypto::bn254_crypto::{
    ArkBlsBn254, ArkBlsBn254Public, ArkBlsBn254Secret, ArkBlsBn254Signature,
};
use gadget_crypto::{KeyEncoding, KeyTypeId};

#[async_trait::async_trait]
pub trait Bn254Backend: Send + Sync {
    /// Generate a new BN254 key pair from seed
    fn bls_bn254_generate_new(&self, seed: Option<&[u8]>) -> Result<ArkBlsBn254Public>;

    /// Generate a BN254 key pair from a string seed
    fn bls_bn254_generate_from_string(&self, secret: &str) -> Result<ArkBlsBn254Public>;

    /// Sign a message using BN254 key
    fn bls_bn254_sign(
        &self,
        public: &ArkBlsBn254Public,
        msg: &[u8; 32],
    ) -> Result<Option<ArkBlsBn254Signature>>;

    /// Get the secret key for a BN254 public key
    fn expose_bls_bn254_secret(
        &self,
        public: &ArkBlsBn254Public,
    ) -> Result<Option<ArkBlsBn254Secret>>;

    /// Iterate over all BN254 public keys
    fn iter_bls_bn254(&self) -> Box<dyn Iterator<Item = ArkBlsBn254Public> + '_>;
}

impl Bn254Backend for Keystore {
    fn bls_bn254_generate_new(&self, seed: Option<&[u8]>) -> Result<ArkBlsBn254Public> {
        let secret =
            ArkBlsBn254::generate_with_seed(seed).map_err(|e| Error::Other(e.to_string()))?;
        insert_bls_bn254_key(self, &secret)
    }

    fn bls_bn254_generate_from_string(&self, seed_string: &str) -> Result<ArkBlsBn254Public> {
        let secret = ArkBlsBn254::generate_with_string(seed_string.to_string())
            .map_err(|e| Error::Other(e.to_string()))?;
        insert_bls_bn254_key(self, &secret)
    }

    fn bls_bn254_sign(
        &self,
        public: &ArkBlsBn254Public,
        msg: &[u8; 32],
    ) -> Result<Option<ArkBlsBn254Signature>> {
        if let Some(mut secret) = self.expose_bls_bn254_secret(public)? {
            Ok(Some(
                ArkBlsBn254::sign_with_secret(&mut secret, msg)
                    .map_err(|e| Error::Other(e.to_string()))?,
            ))
        } else {
            Ok(None)
        }
    }

    fn expose_bls_bn254_secret(
        &self,
        public: &ArkBlsBn254Public,
    ) -> Result<Option<ArkBlsBn254Secret>> {
        let public_bytes = public.to_bytes();

        if let Some(storages) = self.storages.get(&KeyTypeId::Bn254) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_secret_raw(KeyTypeId::Bn254, public_bytes.clone())?
                {
                    return Ok(Some(ArkBlsBn254Secret::from_bytes(&secret_bytes)?));
                }
            }
        }

        Ok(None)
    }

    fn iter_bls_bn254(&self) -> Box<dyn Iterator<Item = ArkBlsBn254Public> + '_> {
        let Some(storages) = self.storages.get(&KeyTypeId::Bn254) else {
            return Box::new(std::iter::empty());
        };

        let mut keys = Vec::new();
        for entry in storages {
            let mut storage_keys = entry
                .storage
                .list_raw(KeyTypeId::Bn254)
                .filter_map(|bytes| ArkBlsBn254Public::from_bytes(&bytes).ok())
                .collect::<Vec<_>>();
            keys.append(&mut storage_keys);
        }
        Box::new(keys.into_iter())
    }
}

fn insert_bls_bn254_key(
    keystore: &Keystore,
    secret: &ArkBlsBn254Secret,
) -> Result<ArkBlsBn254Public> {
    let public = ArkBlsBn254::public_from_secret(secret);
    let public_bytes = public.to_bytes();
    let secret_bytes = secret.to_bytes();

    if let Some(storages) = keystore.storages.get(&KeyTypeId::Bn254) {
        for entry in storages {
            entry.storage.store_raw(
                KeyTypeId::Bn254,
                public_bytes.clone(),
                secret_bytes.clone(),
            )?;
        }
    }

    Ok(public)
}
