use crate::error::{Error, Result};
use crate::keystore::Keystore;
use gadget_crypto::sp_core::{
    SpEcdsaPair, SpEcdsaPublic, SpEd25519Pair, SpEd25519Public, SpSr25519Pair, SpSr25519Public,
};
use gadget_crypto::tangle_pair_signer::TanglePairSigner;
use gadget_crypto::{KeyEncoding, KeyTypeId};
use sp_core::Pair;
use sp_core::{ecdsa, ed25519, sr25519};

#[async_trait::async_trait]
pub trait TangleBackend: Send + Sync {
    // Key Generation Methods
    fn sr25519_generate_new(&self, seed: Option<&[u8]>) -> Result<sr25519::Public>;
    fn ed25519_generate_new(&self, seed: Option<&[u8]>) -> Result<ed25519::Public>;
    fn ecdsa_generate_new(&self, seed: Option<&[u8]>) -> Result<ecdsa::Public>;

    // String-based Key Generation
    fn ecdsa_generate_from_string(&self, string: &str) -> Result<ecdsa::Public>;
    fn ed25519_generate_from_string(&self, string: &str) -> Result<ed25519::Public>;
    fn sr25519_generate_from_string(&self, string: &str) -> Result<sr25519::Public>;

    // Signing Methods
    fn sr25519_sign(
        &self,
        public: &sr25519::Public,
        msg: &[u8],
    ) -> Result<Option<sr25519::Signature>>;
    fn ed25519_sign(
        &self,
        public: &ed25519::Public,
        msg: &[u8],
    ) -> Result<Option<ed25519::Signature>>;
    fn ecdsa_sign(&self, public: &ecdsa::Public, msg: &[u8]) -> Result<Option<ecdsa::Signature>>;

    // Secret Key Access
    fn expose_sr25519_secret(&self, public: &sr25519::Public) -> Result<Option<sr25519::Pair>>;
    fn expose_ed25519_secret(&self, public: &ed25519::Public) -> Result<Option<ed25519::Pair>>;
    fn expose_ecdsa_secret(&self, public: &ecdsa::Public) -> Result<Option<ecdsa::Pair>>;

    // Key Iteration
    fn iter_sr25519(&self) -> Box<dyn Iterator<Item = sr25519::Public> + '_>;
    fn iter_ed25519(&self) -> Box<dyn Iterator<Item = ed25519::Public> + '_>;
    fn iter_ecdsa(&self) -> Box<dyn Iterator<Item = ecdsa::Public> + '_>;

    // Pair Creation Methods
    fn create_sr25519_from_pair<T: Into<sr25519::Pair>>(
        &self,
        pair: T,
    ) -> Result<TanglePairSigner<sr25519::Pair>> {
        let pair = pair.into();
        let seed = pair.as_ref().secret.to_bytes();
        let _ = self.sr25519_generate_new(Some(&seed))?;
        Ok(TanglePairSigner::new(sr25519::Pair::from_seed_slice(
            &seed,
        )?))
    }

    fn create_ed25519_from_pair<T: Into<ed25519::Pair>>(
        &self,
        pair: T,
    ) -> Result<TanglePairSigner<ed25519::Pair>> {
        let pair = pair.into();
        let seed = pair.seed();
        let _ = self.ed25519_generate_new(Some(&seed))?;
        Ok(TanglePairSigner::new(ed25519::Pair::from_seed_slice(
            &seed,
        )?))
    }

    fn create_ecdsa_from_pair<T: Into<ecdsa::Pair>>(
        &self,
        pair: T,
    ) -> Result<TanglePairSigner<ecdsa::Pair>> {
        let pair = pair.into();
        let seed = pair.seed();
        let _ = self.ecdsa_generate_new(Some(&seed))?;
        Ok(TanglePairSigner::new(ecdsa::Pair::from_seed_slice(&seed)?))
    }
}

impl TangleBackend for Keystore {
    fn sr25519_generate_new(&self, seed: Option<&[u8]>) -> Result<sr25519::Public> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Sr25519;

        let secret = SpSr25519Pair(
            sr25519::Pair::from_seed_slice(seed.unwrap_or(&[0u8; 32]))
                .map_err(|e| Error::Other(e.to_string()))?,
        );
        let public = SpSr25519Public(secret.0.public());

        // Store in all available storage backends
        let public_bytes = public.to_bytes();
        let secret_bytes = secret.to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                entry
                    .storage
                    .store_raw(KEY_TYPE_ID, public_bytes.clone(), secret_bytes.clone())?;
            }
        }

        Ok(public.0)
    }

    fn ed25519_generate_new(&self, seed: Option<&[u8]>) -> Result<ed25519::Public> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Ed25519;

        let secret = SpEd25519Pair(
            ed25519::Pair::from_seed_slice(seed.unwrap_or(&[0u8; 32]))
                .map_err(|e| Error::Other(e.to_string()))?,
        );
        let public = SpEd25519Public(secret.0.public());

        // Store in all available storage backends
        let public_bytes = public.to_bytes();
        let secret_bytes = secret.to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                entry
                    .storage
                    .store_raw(KEY_TYPE_ID, public_bytes.clone(), secret_bytes.clone())?;
            }
        }

        Ok(public.0)
    }

    fn ecdsa_generate_new(&self, seed: Option<&[u8]>) -> Result<ecdsa::Public> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Ecdsa;

        let secret = SpEcdsaPair(
            ecdsa::Pair::from_seed_slice(seed.unwrap_or(&[0u8; 32]))
                .map_err(|e| Error::Other(e.to_string()))?,
        );
        let public = SpEcdsaPublic(secret.0.public());

        // Store in all available storage backends
        let public_bytes = public.to_bytes();
        let secret_bytes = secret.to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                entry
                    .storage
                    .store_raw(KEY_TYPE_ID, public_bytes.clone(), secret_bytes.clone())?;
            }
        }

        Ok(public.0)
    }

    fn ecdsa_generate_from_string(&self, string: &str) -> Result<ecdsa::Public> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Ecdsa;

        let secret = SpEcdsaPair(
            ecdsa::Pair::from_string(string, None).map_err(|e| Error::Other(e.to_string()))?,
        );
        let public = SpEcdsaPublic(secret.0.public());

        // Store in all available storage backends
        let public_bytes = public.to_bytes();
        let secret_bytes = secret.to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                entry
                    .storage
                    .store_raw(KEY_TYPE_ID, public_bytes.clone(), secret_bytes.clone())?;
            }
        }

        Ok(public.0)
    }

    fn ed25519_generate_from_string(&self, string: &str) -> Result<ed25519::Public> {
        let seed = if string.as_bytes().len() == 32 {
            string.as_bytes().to_vec()
        } else {
            blake3::hash(string.as_bytes()).as_bytes().to_vec()
        };
        self.ed25519_generate_new(Some(&seed))
    }

    fn sr25519_generate_from_string(&self, string: &str) -> Result<sr25519::Public> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Sr25519;

        let secret = SpSr25519Pair(
            sr25519::Pair::from_string(string, None).map_err(|e| Error::Other(e.to_string()))?,
        );
        let public = SpSr25519Public(secret.0.public());

        // Store in all available storage backends
        let public_bytes = public.to_bytes();
        let secret_bytes = secret.to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                entry
                    .storage
                    .store_raw(KEY_TYPE_ID, public_bytes.clone(), secret_bytes.clone())?;
            }
        }

        Ok(public.0)
    }

    fn sr25519_sign(
        &self,
        public: &sr25519::Public,
        msg: &[u8],
    ) -> Result<Option<sr25519::Signature>> {
        if let Some(secret) = self.expose_sr25519_secret(public)? {
            Ok(Some(secret.sign(msg)))
        } else {
            Ok(None)
        }
    }

    fn ed25519_sign(
        &self,
        public: &ed25519::Public,
        msg: &[u8],
    ) -> Result<Option<ed25519::Signature>> {
        if let Some(secret) = self.expose_ed25519_secret(public)? {
            Ok(Some(secret.sign(msg)))
        } else {
            Ok(None)
        }
    }

    fn ecdsa_sign(&self, public: &ecdsa::Public, msg: &[u8]) -> Result<Option<ecdsa::Signature>> {
        if let Some(secret) = self.expose_ecdsa_secret(public)? {
            Ok(Some(secret.sign(msg)))
        } else {
            Ok(None)
        }
    }

    fn expose_sr25519_secret(&self, public: &sr25519::Public) -> Result<Option<sr25519::Pair>> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Sr25519;

        let public_bytes = SpSr25519Public(*public).to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_secret_raw(KEY_TYPE_ID, public_bytes.clone())?
                {
                    let SpSr25519Pair(pair) = SpSr25519Pair::from_bytes(&secret_bytes)?;
                    return Ok(Some(pair));
                }
            }
        }

        Ok(None)
    }

    fn expose_ed25519_secret(&self, public: &ed25519::Public) -> Result<Option<ed25519::Pair>> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Ed25519;

        let public_bytes = SpEd25519Public(*public).to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_secret_raw(KEY_TYPE_ID, public_bytes.clone())?
                {
                    let SpEd25519Pair(pair) = SpEd25519Pair::from_bytes(&secret_bytes)?;
                    return Ok(Some(pair));
                }
            }
        }

        Ok(None)
    }

    fn expose_ecdsa_secret(&self, public: &ecdsa::Public) -> Result<Option<ecdsa::Pair>> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Ecdsa;

        let public_bytes = SpEcdsaPublic(*public).to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_secret_raw(KEY_TYPE_ID, public_bytes.clone())?
                {
                    let SpEcdsaPair(pair) = SpEcdsaPair::from_bytes(&secret_bytes)?;
                    return Ok(Some(pair));
                }
            }
        }

        Ok(None)
    }

    fn iter_sr25519(&self) -> Box<dyn Iterator<Item = sr25519::Public> + '_> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Sr25519;

        let Some(storages) = self.storages.get(&KEY_TYPE_ID) else {
            return Box::new(std::iter::empty());
        };

        let mut keys = Vec::new();
        for entry in storages {
            let mut storage_keys = entry
                .storage
                .list_raw(KEY_TYPE_ID)
                .filter_map(|bytes| {
                    SpSr25519Public::from_bytes(&bytes)
                        .ok()
                        .map(|public| public.0)
                })
                .collect::<Vec<_>>();
            keys.append(&mut storage_keys);
        }
        Box::new(keys.into_iter())
    }

    fn iter_ed25519(&self) -> Box<dyn Iterator<Item = ed25519::Public> + '_> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Ed25519;

        let Some(storages) = self.storages.get(&KEY_TYPE_ID) else {
            return Box::new(std::iter::empty());
        };

        let mut keys = Vec::new();
        for entry in storages {
            let mut storage_keys = entry
                .storage
                .list_raw(KEY_TYPE_ID)
                .filter_map(|bytes| {
                    SpEd25519Public::from_bytes(&bytes)
                        .ok()
                        .map(|public| public.0)
                })
                .collect::<Vec<_>>();
            keys.append(&mut storage_keys);
        }
        Box::new(keys.into_iter())
    }

    fn iter_ecdsa(&self) -> Box<dyn Iterator<Item = ecdsa::Public> + '_> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Ecdsa;

        let Some(storages) = self.storages.get(&KEY_TYPE_ID) else {
            return Box::new(std::iter::empty());
        };

        let mut keys = Vec::new();
        for entry in storages {
            let mut storage_keys = entry
                .storage
                .list_raw(KEY_TYPE_ID)
                .filter_map(|bytes| {
                    SpEcdsaPublic::from_bytes(&bytes)
                        .ok()
                        .map(|public| public.0)
                })
                .collect::<Vec<_>>();
            keys.append(&mut storage_keys);
        }
        Box::new(keys.into_iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KeystoreConfig;

    #[test]
    fn test_sr25519_operations() -> Result<()> {
        let keystore = Keystore::new(KeystoreConfig::new())?;

        // Generate key
        let public = keystore.sr25519_generate_new(None)?;

        // Sign message
        let msg = b"test message";
        let signature = keystore.sr25519_sign(&public, msg)?.unwrap();

        // Verify signature
        assert!(sr25519::Pair::verify(&signature, msg, &public));

        Ok(())
    }

    #[test]
    fn test_ecdsa_operations() -> Result<()> {
        let keystore = Keystore::new(KeystoreConfig::new())?;

        // Generate from string
        let public = keystore.ecdsa_generate_from_string("test seed")?;

        // Sign message
        let msg = b"test message";
        let signature = keystore.ecdsa_sign(&public, msg)?.unwrap();

        // Verify signature
        assert!(ecdsa::Pair::verify(&signature, msg, &public));

        Ok(())
    }
}
