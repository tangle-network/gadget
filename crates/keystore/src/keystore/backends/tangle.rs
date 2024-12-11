use crate::error::{Error, Result};
use crate::key_types::KeyTypeId;
use crate::key_types::{
    SpEcdsaPair, SpEcdsaPublic, SpEd25519Pair, SpEd25519Public, SpSr25519Pair, SpSr25519Public,
};
use crate::keystore::Keystore;
use gadget_std::any::TypeId;
use sp_core::Pair;
use sp_core::{ecdsa, ed25519, sr25519};
use subxt::tx::PairSigner;
use subxt::PolkadotConfig;

pub struct TanglePairSigner<P: Pair> {
    pub pair: subxt::tx::PairSigner<PolkadotConfig, P>,
}

#[async_trait::async_trait]
pub trait TangleBackend: Send + Sync {
    // Key Generation Methods
    fn sr25519_generate_new(&self, seed: Option<&[u8]>) -> Result<sr25519::Public>;
    fn ed25519_generate_new(&self, seed: Option<&[u8]>) -> Result<ed25519::Public>;
    fn ecdsa_generate_new(&self, seed: Option<&[u8]>) -> Result<ecdsa::Public>;
    #[cfg(feature = "sp-core-bls")]
    fn bls381_generate_new(&self, seed: Option<&[u8]>) -> Result<sp_core::bls381::Public>;
    #[cfg(feature = "sp-core-bls")]
    fn bls377_generate_new(&self, seed: Option<&[u8]>) -> Result<sp_core::bls377::Public>;

    // String-based Key Generation
    fn ecdsa_generate_from_string(&self, string: &str) -> Result<ecdsa::Public>;

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

    #[cfg(feature = "sp-core-bls")]
    fn bls381_sign(
        &self,
        public: &sp_core::bls381::Public,
        msg: &[u8],
    ) -> Result<Option<sp_core::bls381::Signature>>;

    #[cfg(feature = "sp-core-bls")]
    fn bls377_sign(
        &self,
        public: &sp_core::bls377::Public,
        msg: &[u8],
    ) -> Result<Option<sp_core::bls377::Signature>>;

    // Secret Key Access
    fn expose_sr25519_secret(&self, public: &sr25519::Public) -> Result<Option<sr25519::Pair>>;

    fn expose_ed25519_secret(&self, public: &ed25519::Public) -> Result<Option<ed25519::Pair>>;

    fn expose_ecdsa_secret(&self, public: &ecdsa::Public) -> Result<Option<ecdsa::Pair>>;

    #[cfg(feature = "sp-core-bls")]
    fn expose_bls381_secret(
        &self,
        public: &sp_core::bls381::Public,
    ) -> Result<Option<sp_core::bls381::Pair>>;

    #[cfg(feature = "sp-core-bls")]
    fn expose_bls377_secret(
        &self,
        public: &sp_core::bls377::Public,
    ) -> Result<Option<sp_core::bls377::Pair>>;

    // Key Iteration
    fn iter_sr25519(&self) -> Box<dyn Iterator<Item = sr25519::Public> + '_>;
    fn iter_ed25519(&self) -> Box<dyn Iterator<Item = ed25519::Public> + '_>;
    fn iter_ecdsa(&self) -> Box<dyn Iterator<Item = ecdsa::Public> + '_>;
    #[cfg(feature = "sp-core-bls")]
    fn iter_bls381(&self) -> Box<dyn Iterator<Item = sp_core::bls381::Public> + '_>;
    #[cfg(feature = "sp-core-bls")]
    fn iter_bls377(&self) -> Box<dyn Iterator<Item = sp_core::bls377::Public> + '_>;

    // Pair Creation Methods
    fn create_sr25519_from_pair<T: Into<sr25519::Pair>>(
        &self,
        pair: T,
    ) -> Result<TanglePairSigner<sr25519::Pair>> {
        let pair = pair.into();
        let seed = pair.as_ref().secret.to_bytes();
        let _ = self.sr25519_generate_new(Some(&seed))?;
        Ok(TanglePairSigner {
            pair: PairSigner::new(sr25519::Pair::from_seed_slice(&seed)?),
        })
    }

    fn create_ed25519_from_pair<T: Into<ed25519::Pair>>(
        &self,
        pair: T,
    ) -> Result<TanglePairSigner<ed25519::Pair>> {
        let pair = pair.into();
        let seed = pair.seed();
        let _ = self.ed25519_generate_new(Some(&seed))?;
        Ok(TanglePairSigner {
            pair: PairSigner::new(ed25519::Pair::from_seed_slice(&seed)?),
        })
    }

    fn create_ecdsa_from_pair<T: Into<ecdsa::Pair>>(
        &self,
        pair: T,
    ) -> Result<TanglePairSigner<ecdsa::Pair>> {
        let pair = pair.into();
        let seed = pair.seed();
        let _ = self.ecdsa_generate_new(Some(&seed))?;
        Ok(TanglePairSigner {
            pair: PairSigner::new(ecdsa::Pair::from_seed_slice(&seed)?),
        })
    }
}

impl TangleBackend for Keystore {
    fn sr25519_generate_new(&self, seed: Option<&[u8]>) -> Result<sr25519::Public> {
        let secret = SpSr25519Pair(
            sr25519::Pair::from_seed_slice(seed.unwrap_or(&[0u8; 32]))
                .map_err(|e| Error::Other(e.to_string()))?,
        );
        let public = SpSr25519Public(secret.0.public());

        // Store in all available storage backends
        let public_bytes = serde_json::to_vec(&public)?;
        let secret_bytes = serde_json::to_vec(&secret)?;

        if let Some(storages) = self.storages.get(&KeyTypeId::SchnorrkelSr25519) {
            for entry in storages {
                entry.storage.store_raw(
                    TypeId::of::<sr25519::Pair>(),
                    public_bytes.clone(),
                    secret_bytes.clone(),
                )?;
            }
        }

        Ok(public.0)
    }

    fn ed25519_generate_new(&self, seed: Option<&[u8]>) -> Result<ed25519::Public> {
        let secret = SpEd25519Pair(
            ed25519::Pair::from_seed_slice(seed.unwrap_or(&[0u8; 32]))
                .map_err(|e| Error::Other(e.to_string()))?,
        );
        let public = SpEd25519Public(secret.0.public());

        // Store in all available storage backends
        let public_bytes = serde_json::to_vec(&public)?;
        let secret_bytes = serde_json::to_vec(&secret)?;

        if let Some(storages) = self.storages.get(&KeyTypeId::ZebraEd25519) {
            for entry in storages {
                entry.storage.store_raw(
                    TypeId::of::<ed25519::Pair>(),
                    public_bytes.clone(),
                    secret_bytes.clone(),
                )?;
            }
        }

        Ok(public.0)
    }

    fn ecdsa_generate_new(&self, seed: Option<&[u8]>) -> Result<ecdsa::Public> {
        let secret = SpEcdsaPair(
            ecdsa::Pair::from_seed_slice(seed.unwrap_or(&[0u8; 32]))
                .map_err(|e| Error::Other(e.to_string()))?,
        );
        let public = SpEcdsaPublic(secret.0.public());

        // Store in all available storage backends
        let public_bytes = serde_json::to_vec(&public)?;
        let secret_bytes = serde_json::to_vec(&secret)?;

        if let Some(storages) = self.storages.get(&KeyTypeId::K256Ecdsa) {
            for entry in storages {
                entry.storage.store_raw(
                    TypeId::of::<ecdsa::Pair>(),
                    public_bytes.clone(),
                    secret_bytes.clone(),
                )?;
            }
        }

        Ok(public.0)
    }

    #[cfg(feature = "sp-core-bls")]
    fn bls381_generate_new(&self, seed: Option<&[u8]>) -> Result<sp_core::bls381::Public> {
        use crate::key_types::{SpBls381Pair, SpBls381Public};

        let secret = SpBls381Pair(
            sp_core::bls381::Pair::from_seed_slice(seed.unwrap_or(&[0u8; 32]))
                .map_err(|e| Error::Other(e.to_string()))?,
        );
        let public = SpBls381Public(secret.0.public());

        // Store in all available storage backends
        let public_bytes = serde_json::to_vec(&public)?;
        let secret_bytes = serde_json::to_vec(&secret)?;

        if let Some(storages) = self.storages.get(&KeyTypeId::W3fBls381) {
            for entry in storages {
                entry.storage.store_raw(
                    TypeId::of::<sp_core::bls381::Pair>(),
                    public_bytes.clone(),
                    secret_bytes.clone(),
                )?;
            }
        }

        Ok(public.0)
    }

    #[cfg(feature = "sp-core-bls")]
    fn bls377_generate_new(&self, seed: Option<&[u8]>) -> Result<sp_core::bls377::Public> {
        use crate::key_types::{SpBls377Pair, SpBls377Public};

        let secret = SpBls377Pair(
            sp_core::bls377::Pair::from_seed_slice(seed.unwrap_or(&[0u8; 32]))
                .map_err(|e| Error::Other(e.to_string()))?,
        );
        let public = SpBls377Public(secret.0.public());

        // Store in all available storage backends
        let public_bytes = serde_json::to_vec(&public)?;
        let secret_bytes = serde_json::to_vec(&secret)?;

        if let Some(storages) = self.storages.get(&KeyTypeId::W3fBls381) {
            for entry in storages {
                entry.storage.store_raw(
                    TypeId::of::<sp_core::bls377::Pair>(),
                    public_bytes.clone(),
                    secret_bytes.clone(),
                )?;
            }
        }

        Ok(public.0)
    }

    fn ecdsa_generate_from_string(&self, string: &str) -> Result<ecdsa::Public> {
        let seed = blake3::hash(string.as_bytes()).as_bytes().to_vec();
        self.ecdsa_generate_new(Some(&seed))
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

    #[cfg(feature = "sp-core-bls")]
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

    #[cfg(feature = "sp-core-bls")]
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

    fn expose_sr25519_secret(&self, public: &sr25519::Public) -> Result<Option<sr25519::Pair>> {
        let public_bytes = serde_json::to_vec(&SpSr25519Public(*public))?;

        if let Some(storages) = self.storages.get(&KeyTypeId::SchnorrkelSr25519) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_raw(TypeId::of::<sr25519::Pair>(), public_bytes.clone())?
                {
                    let SpSr25519Pair(pair) = serde_json::from_slice(&secret_bytes)?;
                    return Ok(Some(pair));
                }
            }
        }

        Ok(None)
    }

    fn expose_ed25519_secret(&self, public: &ed25519::Public) -> Result<Option<ed25519::Pair>> {
        let public_bytes = serde_json::to_vec(&SpEd25519Public(*public))?;

        if let Some(storages) = self.storages.get(&KeyTypeId::ZebraEd25519) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_raw(TypeId::of::<ed25519::Pair>(), public_bytes.clone())?
                {
                    let SpEd25519Pair(pair) = serde_json::from_slice(&secret_bytes)?;
                    return Ok(Some(pair));
                }
            }
        }

        Ok(None)
    }

    fn expose_ecdsa_secret(&self, public: &ecdsa::Public) -> Result<Option<ecdsa::Pair>> {
        let public_bytes = serde_json::to_vec(&SpEcdsaPublic(*public))?;

        if let Some(storages) = self.storages.get(&KeyTypeId::K256Ecdsa) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_raw(TypeId::of::<ecdsa::Pair>(), public_bytes.clone())?
                {
                    let SpEcdsaPair(pair) = serde_json::from_slice(&secret_bytes)?;
                    return Ok(Some(pair));
                }
            }
        }

        Ok(None)
    }

    #[cfg(feature = "sp-core-bls")]
    fn expose_bls381_secret(
        &self,
        public: &sp_core::bls381::Public,
    ) -> Result<Option<sp_core::bls381::Pair>> {
        use crate::key_types::{SpBls381Pair, SpBls381Public};

        let public_bytes = serde_json::to_vec(&SpBls381Public(*public))?;

        if let Some(storages) = self.storages.get(&KeyTypeId::W3fBls381) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_raw(TypeId::of::<sp_core::bls381::Pair>(), public_bytes.clone())?
                {
                    let SpBls381Pair(pair) = serde_json::from_slice(&secret_bytes)?;
                    return Ok(Some(pair));
                }
            }
        }

        Ok(None)
    }

    #[cfg(feature = "sp-core-bls")]
    fn expose_bls377_secret(
        &self,
        public: &sp_core::bls377::Public,
    ) -> Result<Option<sp_core::bls377::Pair>> {
        use crate::key_types::{SpBls377Pair, SpBls377Public};

        let public_bytes = serde_json::to_vec(&SpBls377Public(*public))?;

        if let Some(storages) = self.storages.get(&KeyTypeId::W3fBls381) {
            for entry in storages {
                if let Some(secret_bytes) = entry
                    .storage
                    .load_raw(TypeId::of::<sp_core::bls377::Pair>(), public_bytes.clone())?
                {
                    let SpBls377Pair(pair) = serde_json::from_slice(&secret_bytes)?;
                    return Ok(Some(pair));
                }
            }
        }

        Ok(None)
    }

    fn iter_sr25519(&self) -> Box<dyn Iterator<Item = sr25519::Public> + '_> {
        if let Some(storages) = self.storages.get(&KeyTypeId::SchnorrkelSr25519) {
            let mut keys = Vec::new();
            for entry in storages {
                let mut storage_keys = entry
                    .storage
                    .list_raw(TypeId::of::<sr25519::Pair>())
                    .filter_map(|bytes| {
                        serde_json::from_slice::<SpSr25519Public>(&bytes)
                            .map(|SpSr25519Public(public)| public)
                            .ok()
                    })
                    .collect::<Vec<_>>();
                keys.append(&mut storage_keys);
            }
            Box::new(keys.into_iter())
        } else {
            Box::new(std::iter::empty())
        }
    }

    fn iter_ed25519(&self) -> Box<dyn Iterator<Item = ed25519::Public> + '_> {
        if let Some(storages) = self.storages.get(&KeyTypeId::ZebraEd25519) {
            let mut keys = Vec::new();
            for entry in storages {
                let mut storage_keys = entry
                    .storage
                    .list_raw(TypeId::of::<ed25519::Pair>())
                    .filter_map(|bytes| {
                        serde_json::from_slice::<SpEd25519Public>(&bytes)
                            .map(|SpEd25519Public(public)| public)
                            .ok()
                    })
                    .collect::<Vec<_>>();
                keys.append(&mut storage_keys);
            }
            Box::new(keys.into_iter())
        } else {
            Box::new(std::iter::empty())
        }
    }

    fn iter_ecdsa(&self) -> Box<dyn Iterator<Item = ecdsa::Public> + '_> {
        if let Some(storages) = self.storages.get(&KeyTypeId::K256Ecdsa) {
            let mut keys = Vec::new();
            for entry in storages {
                let mut storage_keys = entry
                    .storage
                    .list_raw(TypeId::of::<ecdsa::Pair>())
                    .filter_map(|bytes| {
                        serde_json::from_slice::<SpEcdsaPublic>(&bytes)
                            .map(|SpEcdsaPublic(public)| public)
                            .ok()
                    })
                    .collect::<Vec<_>>();
                keys.append(&mut storage_keys);
            }
            Box::new(keys.into_iter())
        } else {
            Box::new(std::iter::empty())
        }
    }

    #[cfg(feature = "sp-core-bls")]
    fn iter_bls381(&self) -> Box<dyn Iterator<Item = sp_core::bls381::Public> + '_> {
        use crate::key_types::SpBls381Public;

        if let Some(storages) = self.storages.get(&KeyTypeId::W3fBls381) {
            let mut keys = Vec::new();
            for entry in storages {
                let mut storage_keys = entry
                    .storage
                    .list_raw(TypeId::of::<sp_core::bls381::Pair>())
                    .filter_map(|bytes| {
                        serde_json::from_slice::<SpBls381Public>(&bytes)
                            .map(|SpBls381Public(public)| public)
                            .ok()
                    })
                    .collect::<Vec<_>>();
                keys.append(&mut storage_keys);
            }
            Box::new(keys.into_iter())
        } else {
            Box::new(std::iter::empty())
        }
    }

    #[cfg(feature = "sp-core-bls")]
    fn iter_bls377(&self) -> Box<dyn Iterator<Item = sp_core::bls377::Public> + '_> {
        use crate::key_types::SpBls377Public;

        if let Some(storages) = self.storages.get(&KeyTypeId::W3fBls381) {
            let mut keys = Vec::new();
            for entry in storages {
                let mut storage_keys = entry
                    .storage
                    .list_raw(TypeId::of::<sp_core::bls377::Pair>())
                    .filter_map(|bytes| {
                        serde_json::from_slice::<SpBls377Public>(&bytes)
                            .map(|SpBls377Public(public)| public)
                            .ok()
                    })
                    .collect::<Vec<_>>();
                keys.append(&mut storage_keys);
            }
            Box::new(keys.into_iter())
        } else {
            Box::new(std::iter::empty())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sr25519_operations() -> Result<()> {
        let keystore = Keystore::new();

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
        let keystore = Keystore::new();

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
