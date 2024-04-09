use color_eyre::Result;
use sp_keystore::{Error, KeystorePtr, Keystore};
use sp_core::{
    crypto::{ByteArray, KeyTypeId, Pair, VrfSecret},
    ecdsa, ed25519, sr25519,
};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use crate::config::KeystoreConfig;

use sp_application_crypto::AppPair;


impl<T: Sized + AsMut<[u8]> + AsRef<[u8]> + Public + Derive> Ss58Codec for T {
    fn from_string(s: &str) -> Result<Self, PublicError> {
        let cap = AddressUri::parse(s)?;
        if cap.pass.is_some() {
            return Err(PublicError::PasswordNotAllowed)
        }
        let s = cap.phrase.unwrap_or(DEV_ADDRESS);
        let addr = if let Some(stripped) = s.strip_prefix("0x") {
            let d = array_bytes::hex2bytes(stripped).map_err(|_| PublicError::InvalidFormat)?;
            Self::from_slice(&d).map_err(|()| PublicError::BadLength)?
        } else {
            Self::from_ss58check(s)?
        };
        if cap.paths.is_empty() {
            Ok(addr)
        } else {
            addr.derive(cap.paths.iter().map(DeriveJunction::from))
                .ok_or(PublicError::InvalidPath)
        }
    }
}

#[derive(Default, Clone)]
pub struct MemoryKeystore {
    /// `KeyTypeId` maps to public keys and public keys map to private keys.
    keys: Arc<RwLock<HashMap<KeyTypeId, HashMap<Vec<u8>, String>>>>,
}

impl MemoryKeystore {
    /// Creates a new instance of `Self`.
    pub fn new() -> Self {
        Self::default()
    }

    fn pair<T: Pair>(&self, key_type: KeyTypeId, public: &T::Public) -> Option<T> {
        self.keys.read().get(&key_type).and_then(|inner| {
            inner
                .get(public.as_slice())
                .map(|s| T::from_string(s, None).expect("seed slice is valid"))
        })
    }

    fn public_keys<T: Pair>(&self, key_type: KeyTypeId) -> Vec<T::Public> {
        self.keys
            .read()
            .get(&key_type)
            .map(|keys| {
                keys.values()
                    .map(|s| T::from_string(s, None).expect("seed slice is valid"))
                    .map(|p| p.public())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn generate_new<T: Pair>(
        &self,
        key_type: KeyTypeId,
        seed: Option<&str>,
    ) -> Result<T::Public, Error> {
        match seed {
            Some(seed) => {
                let pair = T::from_string(seed, None)
                    .map_err(|_| Error::ValidationError("Generates a pair.".to_owned()))?;
                self.keys
                    .write()
                    .entry(key_type)
                    .or_default()
                    .insert(pair.public().to_raw_vec(), seed.into());
                Ok(pair.public())
            },
            None => {
                let (pair, phrase, _) = T::generate_with_phrase(None);
                self.keys
                    .write()
                    .entry(key_type)
                    .or_default()
                    .insert(pair.public().to_raw_vec(), phrase);
                Ok(pair.public())
            },
        }
    }

    fn sign<T: Pair>(
        &self,
        key_type: KeyTypeId,
        public: &T::Public,
        msg: &[u8],
    ) -> Result<Option<T::Signature>, Error> {
        let sig = self.pair::<T>(key_type, public).map(|pair| pair.sign(msg));
        Ok(sig)
    }

    fn vrf_sign<T: Pair + VrfSecret>(
        &self,
        key_type: KeyTypeId,
        public: &T::Public,
        data: &T::VrfSignData,
    ) -> Result<Option<T::VrfSignature>, Error> {
        let sig = self.pair::<T>(key_type, public).map(|pair| pair.vrf_sign(data));
        Ok(sig)
    }

    fn vrf_pre_output<T: Pair + VrfSecret>(
        &self,
        key_type: KeyTypeId,
        public: &T::Public,
        input: &T::VrfInput,
    ) -> Result<Option<T::VrfPreOutput>, Error> {
        let pre_output = self.pair::<T>(key_type, public).map(|pair| pair.vrf_pre_output(input));
        Ok(pre_output)
    }
}

impl Keystore for MemoryKeystore {
    fn sr25519_public_keys(&self, key_type: KeyTypeId) -> Vec<sr25519::Public> {
        self.public_keys::<sr25519::Pair>(key_type)
    }

    fn sr25519_generate_new(
        &self,
        key_type: KeyTypeId,
        seed: Option<&str>,
    ) -> Result<sr25519::Public, Error> {
        self.generate_new::<sr25519::Pair>(key_type, seed)
    }

    fn sr25519_sign(
        &self,
        key_type: KeyTypeId,
        public: &sr25519::Public,
        msg: &[u8],
    ) -> Result<Option<sr25519::Signature>, Error> {
        self.sign::<sr25519::Pair>(key_type, public, msg)
    }

    fn sr25519_vrf_sign(
        &self,
        key_type: KeyTypeId,
        public: &sr25519::Public,
        data: &sr25519::vrf::VrfSignData,
    ) -> Result<Option<sr25519::vrf::VrfSignature>, Error> {
        self.vrf_sign::<sr25519::Pair>(key_type, public, data)
    }

    fn sr25519_vrf_pre_output(
        &self,
        key_type: KeyTypeId,
        public: &sr25519::Public,
        input: &sr25519::vrf::VrfInput,
    ) -> Result<Option<sr25519::vrf::VrfPreOutput>, Error> {
        self.vrf_pre_output::<sr25519::Pair>(key_type, public, input)
    }

    fn ed25519_public_keys(&self, key_type: KeyTypeId) -> Vec<ed25519::Public> {
        self.public_keys::<ed25519::Pair>(key_type)
    }

    fn ed25519_generate_new(
        &self,
        key_type: KeyTypeId,
        seed: Option<&str>,
    ) -> Result<ed25519::Public, Error> {
        self.generate_new::<ed25519::Pair>(key_type, seed)
    }

    fn ed25519_sign(
        &self,
        key_type: KeyTypeId,
        public: &ed25519::Public,
        msg: &[u8],
    ) -> Result<Option<ed25519::Signature>, Error> {
        self.sign::<ed25519::Pair>(key_type, public, msg)
    }

    fn ecdsa_public_keys(&self, key_type: KeyTypeId) -> Vec<ecdsa::Public> {
        self.public_keys::<ecdsa::Pair>(key_type)
    }

    fn ecdsa_generate_new(
        &self,
        key_type: KeyTypeId,
        seed: Option<&str>,
    ) -> Result<ecdsa::Public, Error> {
        self.generate_new::<ecdsa::Pair>(key_type, seed)
    }

    fn ecdsa_sign(
        &self,
        key_type: KeyTypeId,
        public: &ecdsa::Public,
        msg: &[u8],
    ) -> Result<Option<ecdsa::Signature>, Error> {
        self.sign::<ecdsa::Pair>(key_type, public, msg)
    }

    fn ecdsa_sign_prehashed(
        &self,
        key_type: KeyTypeId,
        public: &ecdsa::Public,
        msg: &[u8; 32],
    ) -> Result<Option<ecdsa::Signature>, Error> {
        let sig = self.pair::<ecdsa::Pair>(key_type, public).map(|pair| pair.sign_prehashed(msg));
        Ok(sig)
    }

    fn insert(&self, key_type: KeyTypeId, suri: &str, public: &[u8]) -> Result<(), ()> {
        self.keys
            .write()
            .entry(key_type)
            .or_default()
            .insert(public.to_owned(), suri.to_string());
        Ok(())
    }

    fn keys(&self, key_type: KeyTypeId) -> Result<Vec<Vec<u8>>, Error> {
        let keys = self
            .keys
            .read()
            .get(&key_type)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default();
        Ok(keys)
    }

    fn has_keys(&self, public_keys: &[(Vec<u8>, KeyTypeId)]) -> bool {
        public_keys
            .iter()
            .all(|(k, t)| self.keys.read().get(t).and_then(|s| s.get(k)).is_some())
    }
}

impl Into<KeystorePtr> for MemoryKeystore {
    fn into(self) -> KeystorePtr {
        Arc::new(self)
    }
}



/// Construct a local keystore shareable container
pub struct KeystoreContainer(Arc<MemoryKeystore>);

impl KeystoreContainer {
    /// Construct KeystoreContainer
    pub fn new(config: &KeystoreConfig) -> Result<Self> {
        let keystore = Arc::new(
            MemoryKeystore::new()
            // match config {
            //     KeystoreConfig::Path { path, password } => {
            //         LocalKeystore::open(path.clone(), password.clone())?
            //     }
            //     KeystoreConfig::InMemory => LocalKeystore::in_memory(),
            // }
        );

        Ok(Self(keystore))
    }

    /// Returns a shared reference to a dynamic `Keystore` trait implementation.
    #[allow(dead_code)]
    pub fn keystore(&self) -> KeystorePtr {
        self.0.clone()
    }

    /// Returns a shared reference to the local keystore .
    pub fn local_keystore(&self) -> Arc<MemoryKeystore> {
        self.0.clone()
    }
}
