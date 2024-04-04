use color_eyre::Result;
use sp_keystore::{Error, KeystorePtr, Keystore};
use sp_core::{
    crypto::{ByteArray, KeyTypeId, Pair, VrfSecret},
    ecdsa, ed25519, sr25519,
};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use crate::config::KeystoreConfig;

// use parking_lot::RwLock;
// use sp_application_crypto::{AppCrypto, AppPair, IsWrappedBy};
// use sp_core::{
//     crypto::{ByteArray, ExposeSecret, KeyTypeId, Pair as CorePair, SecretString, VrfSecret},
//     ecdsa, ed25519, sr25519,
// };
// use sp_keystore::{Error as TraitError, Keystore, KeystorePtr};
// use std::{
//     collections::HashMap,
//     // fs::{self, File},
//     // io::Write,
//     path::PathBuf,
//     sync::Arc,
// };
//
//
// use crate::config::KeystoreConfig;
//
// struct KeystoreInner {
//     // path: Option<PathBuf>,
//     additional: HashMap<(KeyTypeId, Vec<u8>), String>,
//     //password: Option<SecretString>,
// }
//
// impl KeystoreInner {
//     /// Open the store at the given path.
//     ///
//     /// Optionally takes a password that will be used to encrypt/decrypt the keys.
//     fn open() {
//     //fn open<T: Into<PathBuf>>(path: T, password: Option<SecretString>) -> Result<Self> {
//     //     let path = path.into();
//     //     fs::create_dir_all(&path)?;
//
//         Ok(Self { additional: HashMap::new() })
//     }
//
//     /// Get the password for this store.
//     fn password(&self) -> Option<&str> {
//         // self.password.as_ref().map(|p| p.expose_secret()).map(|p| p.as_str())
//         None
//     }
//
//     /// Create a new in-memory store.
//     fn new_in_memory(&self) -> Self {
//         Self { additional: HashMap::new() }
//     }
//
//     /// Get the key phrase for the given public key and key type from the in-memory store.
//     fn get_additional_pair(&self) -> Option<&String> {
//     // fn get_additional_pair(&self, public: &[u8], key_type: KeyTypeId) -> Option<&String> {
//         // let key = (key_type, public.to_vec());
//         // self.additional.get(&key)
//         None
//     }
//
//     /// Insert the given public/private key pair with the given key type.
//     ///
//     /// Does not place it into the file system store.
//     fn insert_ephemeral_pair<Pair: CorePair>(
//         &mut self,
//         pair: &Pair,
//         seed: &str,
//         key_type: KeyTypeId,
//     ) {
//         let key = (key_type, pair.public().to_raw_vec());
//         self.additional.insert(key, seed.into());
//     }
//
//     /// Insert a new key with anonymous crypto.
//     ///
//     /// Places it into the file system store, if a path is configured.
//     fn insert(&self, key_type: KeyTypeId, suri: &str, public: &[u8]) -> Result<()> {
//         if let Some(path) = self.key_file_path(public, key_type) {
//             Self::write_to_file(path, suri)?;
//         }
//
//         Ok(())
//     }
//
//     /// Generate a new key.
//     ///
//     /// Places it into the file system store, if a path is configured. Otherwise insert
//     /// it into the memory cache only.
//     fn generate_by_type<Pair: CorePair>(&mut self, key_type: KeyTypeId) -> Result<Pair> {
//         let (pair, phrase, _) = Pair::generate_with_phrase(self.password());
//         if let Some(path) = self.key_file_path(pair.public().as_slice(), key_type) {
//             Self::write_to_file(path, &phrase)?;
//         } else {
//             self.insert_ephemeral_pair(&pair, &phrase, key_type);
//         }
//
//         Ok(pair)
//     }
//
//     /// Write the given `data` to `file`.
//     fn write_to_file(file: PathBuf, data: &str) -> Result<()> {
//         // let mut file = File::create(file)?;
//         //
//         // #[cfg(target_family = "unix")]
//         // {
//         //     use std::os::unix::fs::PermissionsExt;
//         //     file.set_permissions(fs::Permissions::from_mode(0o600))?;
//         // }
//         //
//         // serde_json::to_writer(&file, data)?;
//         // file.flush()?;
//         Ok(())
//     }
//
//     /// Create a new key from seed.
//     ///
//     /// Does not place it into the file system store.
//     fn insert_ephemeral_from_seed_by_type<Pair: CorePair>(
//         &mut self,
//         seed: &str,
//         key_type: KeyTypeId,
//     ) -> Result<Pair> {
//         let pair = Pair::from_string(seed, None).map_err(|_| Error::InvalidSeed)?;
//         self.insert_ephemeral_pair(&pair, seed, key_type);
//         Ok(pair)
//     }
//
//     /// Get the key phrase for a given public key and key type.
//     fn key_phrase_by_type(&self, public: &[u8], key_type: KeyTypeId) -> Result<Option<String>> {
//         // if let Some(phrase) = self.get_additional_pair(public, key_type) {
//         //     return Ok(Some(phrase.clone()))
//         // }
//         //
//         // let path = if let Some(path) = self.key_file_path(public, key_type) {
//         //     path
//         // } else {
//         //     return Ok(None)
//         // };
//         //
//         // if path.exists() {
//         //     let file = File::open(path)?;
//         //
//         //     serde_json::from_reader(&file).map_err(Into::into).map(Some)
//         // } else {
//         //     Ok(None)
//         // }
//         Ok(None)
//     }
//
//     /// Get a key pair for the given public key and key type.
//     fn key_pair_by_type<Pair: CorePair>(
//         &self,
//         public: &Pair::Public,
//         key_type: KeyTypeId,
//     ) -> Result<Option<Pair>> {
//         let phrase = if let Some(p) = self.key_phrase_by_type(public.as_slice(), key_type)? {
//             p
//         } else {
//             return Ok(None)
//         };
//
//         let pair = Pair::from_string(&phrase, self.password()).map_err(|_| Error::InvalidPhrase)?;
//
//         if &pair.public() == public {
//             Ok(Some(pair))
//         } else {
//             Err(Error::PublicKeyMismatch)
//         }
//     }
//
//     /// Get the file path for the given public key and key type.
//     ///
//     /// Returns `None` if the keystore only exists in-memory and there isn't any path to provide.
//     fn key_file_path(&self, public: &[u8], key_type: KeyTypeId) -> Option<PathBuf> {
//         let mut buf = self.path.as_ref()?.clone();
//         let key_type = array_bytes::bytes2hex("", &key_type.0);
//         let key = array_bytes::bytes2hex("", public);
//         buf.push(key_type + key.as_str());
//         Some(buf)
//     }
//
//     /// Returns a list of raw public keys filtered by `KeyTypeId`
//     fn raw_public_keys(&self, key_type: KeyTypeId) -> Result<Vec<Vec<u8>>> {
//         let mut public_keys: Vec<Vec<u8>> = self
//             .additional
//             .keys()
//             .into_iter()
//             .filter_map(|k| if k.0 == key_type { Some(k.1.clone()) } else { None })
//             .collect();
//
//         // if let Some(path) = &self.path {
//         //     for entry in fs::read_dir(&path)? {
//         //         let entry = entry?;
//         //         let path = entry.path();
//         //
//         //         // skip directories and non-unicode file names (hex is unicode)
//         //         if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
//         //             match array_bytes::hex2bytes(name) {
//         //                 Ok(ref hex) if hex.len() > 4 => {
//         //                     if hex[0..4] != key_type.0 {
//         //                         continue
//         //                     }
//         //                     let public = hex[4..].to_vec();
//         //                     public_keys.push(public);
//         //                 },
//         //                 _ => continue,
//         //             }
//         //         }
//         //     }
//         // }
//
//         Ok(public_keys)
//     }
//
//     /// Get a key pair for the given public key.
//     ///
//     /// Returns `Ok(None)` if the key doesn't exist, `Ok(Some(_))` if the key exists or `Err(_)`
//     /// when something failed.
//     pub fn key_pair<Pair: AppPair>(
//         &self,
//         public: &<Pair as AppCrypto>::Public,
//     ) -> Result<Option<Pair>> {
//         self.key_pair_by_type::<Pair::Generic>(IsWrappedBy::from_ref(public), Pair::ID)
//             .map(|v| v.map(Into::into))
//     }
// }
//
// pub struct LocalKeystore(RwLock<KeystoreInner>);
//
// impl LocalKeystore {
//     /// Create a local keystore in memory.
//     pub fn in_memory() -> Self {
//         let inner = KeystoreInner::new_in_memory();
//         Self(RwLock::new(inner))
//     }
//
//     /// Get a key pair for the given public key.
//     ///
//     /// Returns `Ok(None)` if the key doesn't exist, `Ok(Some(_))` if the key exists and
//     /// `Err(_)` when something failed.
//     pub fn key_pair<Pair: AppPair>(
//         &self,
//         public: &<Pair as AppCrypto>::Public,
//     ) -> Result<Option<Pair>> {
//         self.0.read().key_pair::<Pair>(public)
//     }
//
//     fn public_keys<T: CorePair>(&self, key_type: KeyTypeId) -> Vec<T::Public> {
//         self.0
//             .read()
//             .raw_public_keys(key_type)
//             .map(|v| {
//                 v.into_iter().filter_map(|k| T::Public::from_slice(k.as_slice()).ok()).collect()
//             })
//             .unwrap_or_default()
//     }
//
//     fn generate_new<T: CorePair>(
//         &self,
//         key_type: KeyTypeId,
//         seed: Option<&str>,
//     ) -> std::result::Result<T::Public, TraitError> {
//         let pair = match seed {
//             Some(seed) => self.0.write().insert_ephemeral_from_seed_by_type::<T>(seed, key_type),
//             None => self.0.write().generate_by_type::<T>(key_type),
//         }
//             .map_err(|e| -> TraitError { e.into() })?;
//         Ok(pair.public())
//     }
//
//     fn sign<T: CorePair>(
//         &self,
//         key_type: KeyTypeId,
//         public: &T::Public,
//         msg: &[u8],
//     ) -> std::result::Result<Option<T::Signature>, TraitError> {
//         let signature = self
//             .0
//             .read()
//             .key_pair_by_type::<T>(public, key_type)?
//             .map(|pair| pair.sign(msg));
//         Ok(signature)
//     }
//
//     // fn vrf_sign<T: CorePair + VrfSecret>(
//     //     &self,
//     //     key_type: KeyTypeId,
//     //     public: &T::Public,
//     //     data: &T::VrfSignData,
//     // ) -> std::result::Result<Option<T::VrfSignature>, TraitError> {
//     //     let sig = self
//     //         .0
//     //         .read()
//     //         .key_pair_by_type::<T>(public, key_type)?
//     //         .map(|pair| pair.vrf_sign(data));
//     //     Ok(sig)
//     // }
//     //
//     // fn vrf_pre_output<T: CorePair + VrfSecret>(
//     //     &self,
//     //     key_type: KeyTypeId,
//     //     public: &T::Public,
//     //     input: &T::VrfInput,
//     // ) -> std::result::Result<Option<T::VrfPreOutput>, TraitError> {
//     //     let pre_output = self
//     //         .0
//     //         .read()
//     //         .key_pair_by_type::<T>(public, key_type)?
//     //         .map(|pair| pair.vrf_pre_output(input));
//     //     Ok(pre_output)
//     // }
// }

// TODO: Remove above


// /// A keystore implementation usable in tests.
// #[derive(Default, Clone)]
// pub struct MemoryKeystore {
//     /// `KeyTypeId` maps to public keys and public keys map to private keys.
//     keys: Arc<RwLock<HashMap<KeyTypeId, HashMap<Vec<u8>, String>>>>,
// }
//
// impl MemoryKeystore {
//     /// Creates a new instance of `Self`.
//     pub fn new() -> Self {
//         Self::default()
//     }
//
//     fn pair<T: Pair>(&self, key_type: KeyTypeId, public: &T::Public) -> Option<T> {
//         self.keys.read().get(&key_type).and_then(|inner| {
//             inner
//                 .get(public.as_slice())
//                 .map(|s| T::from_string(s, None).expect("seed slice is valid"))
//         })
//     }
//
//     fn public_keys<T: Pair>(&self, key_type: KeyTypeId) -> Vec<T::Public> {
//         self.keys
//             .read()
//             .get(&key_type)
//             .map(|keys| {
//                 keys.values()
//                     .map(|s| T::from_string(s, None).expect("seed slice is valid"))
//                     .map(|p| p.public())
//                     .collect()
//             })
//             .unwrap_or_default()
//     }

    // fn generate_new<T: Pair>(
    //     &self,
    //     key_type: KeyTypeId,
    //     seed: Option<&str>,
    // ) -> Result<T::Public, Error> {
    //     match seed {
    //         Some(seed) => {
    //             let pair = T::from_string(seed, None)
    //                 .map_err(|_| Error::ValidationError("Generates a pair.".to_owned()))?;
    //             self.keys
    //                 .write()
    //                 .entry(key_type)
    //                 .or_default()
    //                 .insert(pair.public().to_raw_vec(), seed.into());
    //             Ok(pair.public())
    //         },
    //         None => {
    //             let (pair, phrase, _) = T::generate_with_phrase(None);
    //             self.keys
    //                 .write()
    //                 .entry(key_type)
    //                 .or_default()
    //                 .insert(pair.public().to_raw_vec(), phrase);
    //             Ok(pair.public())
    //         },
    //     }
    // }

//     fn sign<T: Pair>(
//         &self,
//         key_type: KeyTypeId,
//         public: &T::Public,
//         msg: &[u8],
//     ) -> Result<Option<T::Signature>, Error> {
//         let sig = self.pair::<T>(key_type, public).map(|pair| pair.sign(msg));
//         Ok(sig)
//     }
//
//     fn vrf_sign<T: Pair + VrfSecret>(
//         &self,
//         key_type: KeyTypeId,
//         public: &T::Public,
//         data: &T::VrfSignData,
//     ) -> Result<Option<T::VrfSignature>, Error> {
//         let sig = self.pair::<T>(key_type, public).map(|pair| pair.vrf_sign(data));
//         Ok(sig)
//     }
//
//     fn vrf_pre_output<T: Pair + VrfSecret>(
//         &self,
//         key_type: KeyTypeId,
//         public: &T::Public,
//         input: &T::VrfInput,
//     ) -> Result<Option<T::VrfPreOutput>, Error> {
//         let pre_output = self.pair::<T>(key_type, public).map(|pair| pair.vrf_pre_output(input));
//         Ok(pre_output)
//     }
// }
//
// impl Keystore for MemoryKeystore {
//     fn sr25519_public_keys(&self, key_type: KeyTypeId) -> Vec<sr25519::Public> {
//         self.public_keys::<sr25519::Pair>(key_type)
//     }
//
//     fn sr25519_generate_new(
//         &self,
//         key_type: KeyTypeId,
//         seed: Option<&str>,
//     ) -> Result<sr25519::Public, Error> {
//         self.generate_new::<sr25519::Pair>(key_type, seed)
//     }
//
//     fn sr25519_sign(
//         &self,
//         key_type: KeyTypeId,
//         public: &sr25519::Public,
//         msg: &[u8],
//     ) -> Result<Option<sr25519::Signature>, Error> {
//         self.sign::<sr25519::Pair>(key_type, public, msg)
//     }
//
//     fn sr25519_vrf_sign(
//         &self,
//         key_type: KeyTypeId,
//         public: &sr25519::Public,
//         data: &sr25519::vrf::VrfSignData,
//     ) -> Result<Option<sr25519::vrf::VrfSignature>, Error> {
//         self.vrf_sign::<sr25519::Pair>(key_type, public, data)
//     }
//
//     fn sr25519_vrf_pre_output(
//         &self,
//         key_type: KeyTypeId,
//         public: &sr25519::Public,
//         input: &sr25519::vrf::VrfInput,
//     ) -> Result<Option<sr25519::vrf::VrfPreOutput>, Error> {
//         self.vrf_pre_output::<sr25519::Pair>(key_type, public, input)
//     }
//
//     fn ed25519_public_keys(&self, key_type: KeyTypeId) -> Vec<ed25519::Public> {
//         self.public_keys::<ed25519::Pair>(key_type)
//     }
//
//     fn ed25519_generate_new(
//         &self,
//         key_type: KeyTypeId,
//         seed: Option<&str>,
//     ) -> Result<ed25519::Public, Error> {
//         self.generate_new::<ed25519::Pair>(key_type, seed)
//     }
//
//     fn ed25519_sign(
//         &self,
//         key_type: KeyTypeId,
//         public: &ed25519::Public,
//         msg: &[u8],
//     ) -> Result<Option<ed25519::Signature>, Error> {
//         self.sign::<ed25519::Pair>(key_type, public, msg)
//     }
//
//     fn ecdsa_public_keys(&self, key_type: KeyTypeId) -> Vec<ecdsa::Public> {
//         self.public_keys::<ecdsa::Pair>(key_type)
//     }
//
//     fn ecdsa_generate_new(
//         &self,
//         key_type: KeyTypeId,
//         seed: Option<&str>,
//     ) -> Result<ecdsa::Public, Error> {
//         self.generate_new::<ecdsa::Pair>(key_type, seed)
//     }
//
//     fn ecdsa_sign(
//         &self,
//         key_type: KeyTypeId,
//         public: &ecdsa::Public,
//         msg: &[u8],
//     ) -> Result<Option<ecdsa::Signature>, Error> {
//         self.sign::<ecdsa::Pair>(key_type, public, msg)
//     }
//
//     fn ecdsa_sign_prehashed(
//         &self,
//         key_type: KeyTypeId,
//         public: &ecdsa::Public,
//         msg: &[u8; 32],
//     ) -> Result<Option<ecdsa::Signature>, Error> {
//         let sig = self.pair::<ecdsa::Pair>(key_type, public).map(|pair| pair.sign_prehashed(msg));
//         Ok(sig)
//     }
//
//     fn insert(&self, key_type: KeyTypeId, suri: &str, public: &[u8]) -> Result<(), ()> {
//         self.keys
//             .write()
//             .entry(key_type)
//             .or_default()
//             .insert(public.to_owned(), suri.to_string());
//         Ok(())
//     }
//
//     fn keys(&self, key_type: KeyTypeId) -> Result<Vec<Vec<u8>>, Error> {
//         let keys = self
//             .keys
//             .read()
//             .get(&key_type)
//             .map(|map| map.keys().cloned().collect())
//             .unwrap_or_default();
//         Ok(keys)
//     }
//
//     fn has_keys(&self, public_keys: &[(Vec<u8>, KeyTypeId)]) -> bool {
//         public_keys
//             .iter()
//             .all(|(k, t)| self.keys.read().get(t).and_then(|s| s.get(k)).is_some())
//     }
// }
//
// impl Into<KeystorePtr> for MemoryKeystore {
//     fn into(self) -> KeystorePtr {
//         Arc::new(self)
//     }
// }



// /// Construct a local keystore shareable container
// pub struct KeystoreContainer(Arc<MemoryKeystore>);
//
// impl KeystoreContainer {
//     /// Construct KeystoreContainer
//     pub fn new(config: &KeystoreConfig) -> Result<Self> {
//         let keystore = Arc::new(
//             MemoryKeystore::new()
//             // match config {
//             //     KeystoreConfig::Path { path, password } => {
//             //         LocalKeystore::open(path.clone(), password.clone())?
//             //     }
//             //     KeystoreConfig::InMemory => LocalKeystore::in_memory(),
//             // }
//         );
//
//         Ok(Self(keystore))
//     }
//
//     /// Returns a shared reference to a dynamic `Keystore` trait implementation.
//     #[allow(dead_code)]
//     pub fn keystore(&self) -> KeystorePtr {
//         self.0.clone()
//     }
//
//     /// Returns a shared reference to the local keystore .
//     pub fn local_keystore(&self) -> Arc<MemoryKeystore> {
//         self.0.clone()
//     }
// }
