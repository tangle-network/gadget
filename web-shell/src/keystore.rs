use color_eyre::Result;
use sp_keystore::KeystorePtr;
use std::sync::Arc;

use crate::config::KeystoreConfig;

// struct KeystoreInner {
//     // path: Option<PathBuf>,
//     additional: HashMap<(KeyTypeId, Vec<u8>), String>,
//     //password: Option<SecretString>,
// }

pub struct LocalKeystore(RwLock<KeystoreInner>);

impl LocalKeystore {
    /// Create a local keystore in memory.
    pub fn in_memory() -> Self {
        let inner = KeystoreInner::new_in_memory();
        Self(RwLock::new(inner))
    }

    /// Get a key pair for the given public key.
    ///
    /// Returns `Ok(None)` if the key doesn't exist, `Ok(Some(_))` if the key exists and
    /// `Err(_)` when something failed.
    pub fn key_pair<Pair: AppPair>(
        &self,
        public: &<Pair as AppCrypto>::Public,
    ) -> Result<Option<Pair>> {
        self.0.read().key_pair::<Pair>(public)
    }

    fn public_keys<T: CorePair>(&self, key_type: KeyTypeId) -> Vec<T::Public> {
        self.0
            .read()
            .raw_public_keys(key_type)
            .map(|v| {
                v.into_iter().filter_map(|k| T::Public::from_slice(k.as_slice()).ok()).collect()
            })
            .unwrap_or_default()
    }

    fn generate_new<T: CorePair>(
        &self,
        key_type: KeyTypeId,
        seed: Option<&str>,
    ) -> std::result::Result<T::Public, TraitError> {
        let pair = match seed {
            Some(seed) => self.0.write().insert_ephemeral_from_seed_by_type::<T>(seed, key_type),
            None => self.0.write().generate_by_type::<T>(key_type),
        }
            .map_err(|e| -> TraitError { e.into() })?;
        Ok(pair.public())
    }

    fn sign<T: CorePair>(
        &self,
        key_type: KeyTypeId,
        public: &T::Public,
        msg: &[u8],
    ) -> std::result::Result<Option<T::Signature>, TraitError> {
        let signature = self
            .0
            .read()
            .key_pair_by_type::<T>(public, key_type)?
            .map(|pair| pair.sign(msg));
        Ok(signature)
    }

    fn vrf_sign<T: CorePair + VrfSecret>(
        &self,
        key_type: KeyTypeId,
        public: &T::Public,
        data: &T::VrfSignData,
    ) -> std::result::Result<Option<T::VrfSignature>, TraitError> {
        let sig = self
            .0
            .read()
            .key_pair_by_type::<T>(public, key_type)?
            .map(|pair| pair.vrf_sign(data));
        Ok(sig)
    }

    fn vrf_pre_output<T: CorePair + VrfSecret>(
        &self,
        key_type: KeyTypeId,
        public: &T::Public,
        input: &T::VrfInput,
    ) -> std::result::Result<Option<T::VrfPreOutput>, TraitError> {
        let pre_output = self
            .0
            .read()
            .key_pair_by_type::<T>(public, key_type)?
            .map(|pair| pair.vrf_pre_output(input));
        Ok(pre_output)
    }
}



/// Construct a local keystore shareable container
pub struct KeystoreContainer(Arc<LocalKeystore>);

impl KeystoreContainer {
    /// Construct KeystoreContainer
    pub fn new(config: &KeystoreConfig) -> Result<Self> {
        let keystore = Arc::new(
            LocalKeystore::in_memory()
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
    pub fn local_keystore(&self) -> Arc<LocalKeystore> {
        self.0.clone()
    }
}
