use crate::error::{Error, Result};
use crate::keystore::backends::tangle::TangleBackend;
use crate::keystore::Keystore;
use gadget_crypto::sp_core::{SpBls377Pair, SpBls377Public, SpBls381Pair, SpBls381Public};
use gadget_crypto::{KeyEncoding, KeyTypeId};
use gadget_std::rand::RngCore;
use sp_core::Pair;

#[async_trait::async_trait]
pub trait TangleBlsBackend: TangleBackend {
    // BLS Key Generation Methods
    fn bls381_generate_new(&self, seed: Option<&[u8]>) -> Result<sp_core::bls381::Public>;
    fn bls377_generate_new(&self, seed: Option<&[u8]>) -> Result<sp_core::bls377::Public>;

    // BLS Signing Methods
    fn bls381_sign(
        &self,
        public: &sp_core::bls381::Public,
        msg: &[u8],
    ) -> Result<Option<sp_core::bls381::Signature>>;

    fn bls377_sign(
        &self,
        public: &sp_core::bls377::Public,
        msg: &[u8],
    ) -> Result<Option<sp_core::bls377::Signature>>;

    // BLS Secret Key Access
    fn expose_bls381_secret(
        &self,
        public: &sp_core::bls381::Public,
    ) -> Result<Option<SpBls381Pair>>;

    fn expose_bls377_secret(
        &self,
        public: &sp_core::bls377::Public,
    ) -> Result<Option<SpBls377Pair>>;

    // BLS Key Iteration
    fn iter_bls381(&self) -> Box<dyn Iterator<Item = sp_core::bls381::Public> + '_>;
    fn iter_bls377(&self) -> Box<dyn Iterator<Item = sp_core::bls377::Public> + '_>;
}

impl TangleBlsBackend for Keystore {
    fn bls381_generate_new(&self, seed: Option<&[u8]>) -> Result<sp_core::bls381::Public> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Bls381;

        let mut final_seed = [0; 32];
        match seed {
            Some(seed) => final_seed.copy_from_slice(seed),
            None => {
                let mut seed = [0; 32];
                gadget_std::rand::thread_rng().fill_bytes(&mut seed[..]);
                final_seed = seed
            }
        };

        let pair = SpBls381Pair::from_bytes(final_seed.as_slice())
            .map_err(|e| Error::Other(e.to_string()))?;
        let public = pair.public();

        // Store in all available storage backends
        let public_bytes = public.to_bytes();
        let secret_bytes = pair.to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                entry
                    .storage
                    .store_raw(KEY_TYPE_ID, public_bytes.clone(), secret_bytes.clone())?;
            }
        }

        Ok(public.0)
    }

    fn bls377_generate_new(&self, seed: Option<&[u8]>) -> Result<sp_core::bls377::Public> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Bls377;

        let mut final_seed = [0; 32];
        match seed {
            Some(seed) => final_seed.copy_from_slice(seed),
            None => {
                let mut seed = [0; 32];
                gadget_std::rand::thread_rng().fill_bytes(&mut seed[..]);
                final_seed = seed
            }
        };

        let pair = SpBls377Pair::from_bytes(final_seed.as_slice())
            .map_err(|e| Error::Other(e.to_string()))?;
        let public = pair.public();

        // Store in all available storage backends
        let public_bytes = public.to_bytes();
        let secret_bytes = pair.to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                entry
                    .storage
                    .store_raw(KEY_TYPE_ID, public_bytes.clone(), secret_bytes.clone())?;
            }
        }

        Ok(public.0)
    }

    fn bls381_sign(
        &self,
        public: &sp_core::bls381::Public,
        msg: &[u8],
    ) -> Result<Option<sp_core::bls381::Signature>> {
        if let Some(secret) = self.expose_bls381_secret(public)? {
            Ok(Some(secret.sign(msg)))
        } else {
            Ok(None)
        }
    }

    fn bls377_sign(
        &self,
        public: &sp_core::bls377::Public,
        msg: &[u8],
    ) -> Result<Option<sp_core::bls377::Signature>> {
        if let Some(secret) = self.expose_bls377_secret(public)? {
            Ok(Some(secret.sign(msg)))
        } else {
            Ok(None)
        }
    }

    fn expose_bls381_secret(
        &self,
        public: &sp_core::bls381::Public,
    ) -> Result<Option<SpBls381Pair>> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Bls381;

        let public_bytes = SpBls381Public(*public).to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_secret_raw(KEY_TYPE_ID, public_bytes.clone())?
                {
                    let pair = SpBls381Pair::from_bytes(&secret_bytes)?;
                    return Ok(Some(pair));
                }
            }
        }

        Ok(None)
    }

    fn expose_bls377_secret(
        &self,
        public: &sp_core::bls377::Public,
    ) -> Result<Option<SpBls377Pair>> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Bls377;

        let public_bytes = SpBls377Public(*public).to_bytes();

        if let Some(storages) = self.storages.get(&KEY_TYPE_ID) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_secret_raw(KEY_TYPE_ID, public_bytes.clone())?
                {
                    let pair = SpBls377Pair::from_bytes(&secret_bytes)?;
                    return Ok(Some(pair));
                }
            }
        }

        Ok(None)
    }

    fn iter_bls381(&self) -> Box<dyn Iterator<Item = sp_core::bls381::Public> + '_> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Bls381;

        let Some(storages) = self.storages.get(&KEY_TYPE_ID) else {
            return Box::new(std::iter::empty());
        };

        let mut keys = Vec::new();
        for entry in storages {
            let mut storage_keys = entry
                .storage
                .list_raw(KEY_TYPE_ID)
                .filter_map(|bytes| {
                    SpBls381Public::from_bytes(&bytes)
                        .ok()
                        .map(|public| public.0)
                })
                .collect::<Vec<_>>();
            keys.append(&mut storage_keys);
        }
        Box::new(keys.into_iter())
    }

    fn iter_bls377(&self) -> Box<dyn Iterator<Item = sp_core::bls377::Public> + '_> {
        const KEY_TYPE_ID: KeyTypeId = KeyTypeId::Bls377;

        let Some(storages) = self.storages.get(&KEY_TYPE_ID) else {
            return Box::new(std::iter::empty());
        };

        let mut keys = Vec::new();
        for entry in storages {
            let mut storage_keys = entry
                .storage
                .list_raw(KEY_TYPE_ID)
                .filter_map(|bytes| {
                    SpBls377Public::from_bytes(&bytes)
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
    fn test_bls381_operations() -> Result<()> {
        let keystore = Keystore::new(KeystoreConfig::new())?;

        // Generate key
        let public = keystore.bls381_generate_new(None)?;

        // Sign message
        let msg = b"test message";
        let signature = keystore.bls381_sign(&public, msg)?.unwrap();
        // Verify signature
        assert!(sp_core::bls381::Pair::verify(&signature, msg, &public));

        Ok(())
    }

    #[test]
    fn test_bls377_operations() -> Result<()> {
        let keystore = Keystore::new(KeystoreConfig::new())?;

        // Generate key
        let public = keystore.bls377_generate_new(None)?;

        // Sign message
        let msg = b"test message";
        let signature = keystore.bls377_sign(&public, msg)?.unwrap();
        // Verify signature
        assert!(sp_core::bls377::Pair::verify(&signature, msg, &public));

        Ok(())
    }
}
