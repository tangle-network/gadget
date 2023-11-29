// Copyright 2022 Webb Technologies Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::convert::{From, TryInto};
use std::sync::Arc;

use parity_scale_codec::{Decode, Encode};
use sp_application_crypto::{key_types::ACCOUNT, sr25519, RuntimeAppPublic, sp_core};
use sp_keystore::{Keystore, KeystorePtr};

use dkg_runtime_primitives::{
    crypto::{Public, Signature},
    KEY_TYPE,
};
use itertools::Itertools;
use sc_keystore::LocalKeystore;
use sp_application_crypto::sp_core::keccak_256;
use crate::error;
use crate::util::DebugLogger;

/// A DKG specific keystore implemented as a `Newtype`. This is basically a
/// wrapper around [`Keystore`] and allows to customize
/// common cryptographic functionality.
#[derive(Clone)]
pub struct DKGKeystore(Option<KeystorePtr>, DebugLogger);

impl DKGKeystore {
    pub fn new(keystore: Option<Arc<dyn Keystore>>, logger: DebugLogger) -> Self {
        Self(keystore, logger)
    }

    pub fn new_default(logger: DebugLogger) -> Self {
        let keystore = Arc::new(LocalKeystore::in_memory()) as Arc<dyn Keystore>;
        Self::new(Some(keystore), logger)
    }

    pub fn set_logger(&mut self, logger: DebugLogger) {
        self.1 = logger;
    }
    /// Check if the keystore contains a private key for one of the public keys
    /// contained in `keys`. A public key with a matching private key is known
    /// as a local authority id.
    ///
    /// Return the public key for which we also do have a private key. If no
    /// matching private key is found, `None` will be returned.
    pub fn authority_id(&self, keys: &[Public]) -> Option<Public> {
        let store = self.0.clone()?;

        // we do check for multiple private keys as a key store sanity check.
        let mut public: Vec<Public> = keys
            .iter()
            .filter(|k| Keystore::has_keys(&*store, &[(k.to_raw_vec(), KEY_TYPE)]))
            .cloned()
            .unique()
            .collect();

        if public.len() > 1 {
            self.1.warn(format!(
                "🕸️  (authority_id) Multiple private keys found for: {:?} ({})",
                public,
                public.len()
            ));
        }

        public.pop()
    }

    /// Check if the keystore contains a private key for one of the sr25519 public keys
    /// contained in `keys`. A public key with a matching private key is known
    /// as a local authority id.
    ///
    /// Return the public key for which we also do have a private key. If no
    /// matching private key is found, `None` will be returned.
    pub fn sr25519_public_key(&self, keys: &[sr25519::Public]) -> Option<sr25519::Public> {
        let store = self.0.clone()?;

        // we do check for multiple private keys as a key store sanity check.
        let public: Vec<sr25519::Public> = keys
            .iter()
            .filter(|k| Keystore::has_keys(&*store, &[(k.encode(), ACCOUNT)]))
            .cloned()
            .collect();

        if public.len() > 1 {
            self.1.warn(format!(
                "🕸️  (sr25519_public_key) Multiple private keys found for: {:?} ({})",
                public,
                public.len()
            ));
        }

        public.get(0).cloned()
    }

    /// Sign `message` with the `public` key.
    ///
    /// Note that `message` usually will be pre-hashed before being signed.
    ///
    /// Return the message signature or an error in case of failure.
    #[allow(dead_code)]
    pub fn sign(&self, public: &Public, message: &[u8]) -> Result<Signature, error::Error> {
        let store = self.0.clone().ok_or_else(|| error::Error::Keystore("no Keystore".to_string()))?;

        let msg = keccak_256(message);
        let public = public.as_ref();

        let sig = Keystore::ecdsa_sign_prehashed(&*store, KEY_TYPE, public, &msg)
            .map_err(|e| error::Error::Keystore(e.to_string()))?
            .ok_or_else(|| error::Error::Signature("ecdsa_sign_prehashed() failed".to_string()))?;

        // check that `sig` has the expected result type
        let sig = sig.clone().try_into().map_err(|_| {
            error::Error::Signature(format!("invalid signature {sig:?} for key {public:?}"))
        })?;

        Ok(sig)
    }

    /// Returns a vector of [`dkg_runtime_primitives::crypto::Public`] keys which are currently
    /// supported (i.e. found in the keystore).
    pub fn public_keys(&self) -> Result<Vec<Public>, error::Error> {
        let store = self.0.clone().ok_or_else(|| error::Error::Keystore("no Keystore".to_string()))?;

        let pk: Vec<Public> = Keystore::ecdsa_public_keys(&*store, KEY_TYPE)
            .iter()
            .map(|k| Public::from(*k))
            .collect();

        Ok(pk)
    }

    /// Returns a vector of sr25519 Public keys which are currently supported (i.e. found
    /// in the keystore).
    pub fn sr25519_public_keys(&self) -> Result<Vec<sr25519::Public>, error::Error> {
        let store = self.0.clone().ok_or_else(|| error::Error::Keystore("no Keystore".to_string()))?;

        let pk: Vec<sr25519::Public> = Keystore::sr25519_public_keys(&*store, ACCOUNT);

        Ok(pk)
    }

    /// Sign `message` with the sr25519 `public` key.
    ///
    /// Note that `message` usually will be pre-hashed before being signed.
    ///
    /// Return the message signature or an error in case of failure.
    #[allow(dead_code)]
    pub fn sr25519_sign(
        &self,
        public: &sr25519::Public,
        message: &[u8],
    ) -> Result<sr25519::Signature, error::Error> {
        let store = self.0.clone().ok_or_else(|| error::Error::Keystore("no Keystore".to_string()))?;

        let sig = Keystore::sign_with(&*store, ACCOUNT, sr25519::CRYPTO_ID, public, message)
            .map_err(|e| error::Error::Keystore(e.to_string()))?
            .ok_or_else(|| error::Error::Signature("sr25519_sign() failed".to_string()))?;

        // check that `sig` has the expected result type
        let signature: sr25519::Signature =
            sr25519::Signature::decode(&mut &sig[..]).map_err(|_| {
                error::Error::Signature(format!("invalid signature {sig:?} for key {public:?}"))
            })?;

        Ok(signature)
    }

    /// Use the `public` key to verify that `sig` is a valid signature for `message`.
    ///
    /// Return `true` if the signature is authentic, `false` otherwise.
    pub fn verify(public: &Public, sig: &Signature, message: &[u8]) -> bool {
        let msg = keccak_256(message);
        let sig = sig.as_ref();
        let public = public.as_ref();

        sp_core::ecdsa::Pair::verify_prehashed(sig, &msg, public)
    }

    pub fn as_dyn_crypto_store(&self) -> Option<&dyn Keystore> {
        self.0.as_deref()
    }
}

impl From<Option<KeystorePtr>> for DKGKeystore {
    fn from(store: Option<KeystorePtr>) -> Self {
        Self(store, DebugLogger::new("DKGKeystore", None).expect("Should not fail"))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)] // allow unwraps in tests
mod tests {
    use std::sync::Arc;

    use sc_keystore::LocalKeystore;
    use sp_keystore::{Keystore, KeystorePtr};

    use crate::keyring::Keyring;
    use dkg_runtime_primitives::{crypto, KEY_TYPE};

    use super::DKGKeystore;
    use crate::error::Error;

    fn keystore() -> KeystorePtr {
        Arc::new(LocalKeystore::in_memory())
    }

    #[test]
    fn authority_id_works() {
        let store = keystore();

        let alice: crypto::Public =
            Keystore::ecdsa_generate_new(&*store, KEY_TYPE, Some(&Keyring::Alice.to_seed()))
                .ok()
                .unwrap()
                .into();

        let bob = Keyring::Bob.public();
        let charlie = Keyring::Charlie.public();

        let store: DKGKeystore = Some(store).into();

        let mut keys = vec![bob, charlie];

        let id = store.authority_id(keys.as_slice());
        assert!(id.is_none());

        keys.push(alice.clone());

        let id = store.authority_id(keys.as_slice()).unwrap();
        assert_eq!(id, alice);
    }

    #[test]
    fn sign_works() {
        let store = keystore();

        let alice: crypto::Public =
            Keystore::ecdsa_generate_new(&*store, KEY_TYPE, Some(&Keyring::Alice.to_seed()))
                .ok()
                .unwrap()
                .into();

        let store: DKGKeystore = Some(store).into();

        let msg = b"are you involved or commited?";

        let sig1 = store.sign(&alice, msg).unwrap();
        let sig2 = Keyring::Alice.sign(msg);

        assert_eq!(sig1, sig2);
    }

    #[test]
    fn sign_error() {
        let store = keystore();

        let _ = Keystore::ecdsa_generate_new(&*store, KEY_TYPE, Some(&Keyring::Bob.to_seed()))
            .ok()
            .unwrap();

        let store: DKGKeystore = Some(store).into();

        let alice = Keyring::Alice.public();

        let msg = b"are you involved or commited?";
        let sig = store.sign(&alice, msg).err().unwrap();
        let err = Error::Signature("ecdsa_sign_prehashed() failed".to_string());

        assert_eq!(sig, err);
    }

    #[test]
    fn sign_no_keystore() {
        let store: DKGKeystore = None.into();

        let alice = Keyring::Alice.public();
        let msg = b"are you involved or commited";

        let sig = store.sign(&alice, msg).err().unwrap();
        let err = Error::Keystore("no Keystore".to_string());
        assert_eq!(sig, err);
    }

    #[test]
    fn verify_works() {
        let store = keystore();

        let alice: crypto::Public =
            Keystore::ecdsa_generate_new(&*store, KEY_TYPE, Some(&Keyring::Alice.to_seed()))
                .ok()
                .unwrap()
                .into();

        let store: DKGKeystore = Some(store).into();

        // `msg` and `sig` match
        let msg = b"are you involved or commited?";
        let sig = store.sign(&alice, msg).unwrap();
        assert!(DKGKeystore::verify(&alice, &sig, msg));

        // `msg and `sig` don't match
        let msg = b"you are just involved";
        assert!(!DKGKeystore::verify(&alice, &sig, msg));
    }

    // Note that we use keys with and without a seed for this test.
    #[test]
    fn public_keys_works() {
        const TEST_TYPE: sp_application_crypto::KeyTypeId =
            sp_application_crypto::KeyTypeId(*b"test");

        let store = keystore();

        let add_key = |key_type, seed: Option<&str>| {
            Keystore::ecdsa_generate_new(&*store, key_type, seed).unwrap()
        };

        // test keys
        let _ = add_key(TEST_TYPE, Some(Keyring::Alice.to_seed().as_str()));
        let _ = add_key(TEST_TYPE, Some(Keyring::Bob.to_seed().as_str()));

        let _ = add_key(TEST_TYPE, None);
        let _ = add_key(TEST_TYPE, None);

        // DKG keys
        let _ = add_key(KEY_TYPE, Some(Keyring::Dave.to_seed().as_str()));
        let _ = add_key(KEY_TYPE, Some(Keyring::Eve.to_seed().as_str()));

        let key1: crypto::Public = add_key(KEY_TYPE, None).into();
        let key2: crypto::Public = add_key(KEY_TYPE, None).into();

        let store: DKGKeystore = Some(store).into();

        let keys = store.public_keys().ok().unwrap();

        assert_eq!(keys.len(), 4);
        assert!(keys.contains(&Keyring::Dave.public()));
        assert!(keys.contains(&Keyring::Eve.public()));
        assert!(keys.contains(&key1));
        assert!(keys.contains(&key2));
    }
}
