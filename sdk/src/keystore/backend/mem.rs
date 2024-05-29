//! In-Memory Keystore Backend that supports different cryptographic key operations such as key generation, signing, and public key retrieval.

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, sync::Arc};
#[cfg(feature = "std")]
use std::{collections::BTreeMap, sync::Arc};
#[cfg(feature = "keystore-bls381")]
use w3f_bls::SerializableToBytes;

use parking_lot::RwLock;

use crate::keystore::{bls381, ecdsa, ed25519, sr25519, Backend};

/// The type alias for the In Memory `KeyMap`.
type KeyMap<P, S> = Arc<RwLock<BTreeMap<P, S>>>;

#[cfg(feature = "keystore-ed25519")]
#[derive(Debug, Clone)]
struct Ed25519PublicWrapper(ed25519::Public);

#[cfg(feature = "keystore-ed25519")]
impl PartialEq for Ed25519PublicWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref() == other.0.as_ref()
    }
}

#[cfg(feature = "keystore-ed25519")]
impl Eq for Ed25519PublicWrapper {}

#[cfg(feature = "keystore-ed25519")]
impl Ord for Ed25519PublicWrapper {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.as_ref().cmp(other.0.as_ref())
    }
}

#[cfg(feature = "keystore-ed25519")]
impl PartialOrd for Ed25519PublicWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(feature = "keystore-bls381")]
#[derive(Clone, PartialEq, Eq)]
struct Bls381PublicWrapper(bls381::Public);

#[cfg(feature = "keystore-bls381")]
impl PartialOrd for Bls381PublicWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(feature = "keystore-bls381")]
impl Ord for Bls381PublicWrapper {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.to_bytes().cmp(&other.0.to_bytes())
    }
}

/// The In-Memory Keystore Backend. It stores everything in memory
/// and does not persist anything, once dropped all the keys are lost, so use with caution.
/// This is useful for testing and development purposes.
/// It implements the [`crate::keystore::Backend`] trait.
///
/// Note: Cloning this backend is cheap, as it uses [`Arc`] and [`RwLock`] internally.
#[derive(Default, Clone)]
pub struct InMemoryKeystore {
    sr25519: KeyMap<sr25519::Public, sr25519::Secret>,
    ecdsa: KeyMap<ecdsa::Public, ecdsa::Secret>,
    ed25519: KeyMap<Ed25519PublicWrapper, ed25519::Secret>,
    bls381: KeyMap<Bls381PublicWrapper, bls381::Secret>,
}

impl core::fmt::Debug for InMemoryKeystore {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InMemoryKeystore")
            .field("sr25519", &"<hidden>")
            .field("ecdsa", &"<hidden>")
            .field("ed25519", &"<hidden>")
            .field("bls381", &"<hidden>")
            .finish()
    }
}

impl InMemoryKeystore {
    /// Create a new In-Memory Keystore Backend.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Backend for InMemoryKeystore {
    #[cfg(feature = "keystore-sr25519")]
    fn sr25519_generate_new(
        &self,
        seed: Option<&[u8]>,
    ) -> Result<sr25519::Public, crate::keystore::Error> {
        let secret = sr25519::generate_with_optional_seed(seed)?;
        let public = secret.to_public();
        let old = self.sr25519.write().insert(public, secret);
        assert!(old.is_none(), "generated key already exists");
        Ok(public)
    }

    #[cfg(feature = "keystore-sr25519")]
    fn sr25519_sign(
        &self,
        public: &sr25519::Public,
        msg: &[u8],
    ) -> Result<Option<sr25519::Signature>, crate::keystore::Error> {
        let lock = self.sr25519.read();
        let secret = lock.get(public);
        if let Some(secret) = secret {
            Ok(Some(sr25519::sign(secret, msg)?))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "keystore-ed25519")]
    fn ed25519_generate_new(
        &self,
        seed: Option<&[u8]>,
    ) -> Result<ed25519::Public, crate::keystore::Error> {
        let secret = ed25519::generate_with_optional_seed(seed)?;
        let public = ed25519::to_public(&secret);
        let old = self
            .ed25519
            .write()
            .insert(Ed25519PublicWrapper(public), secret);
        assert!(old.is_none(), "generated key already exists");
        Ok(public)
    }

    #[cfg(feature = "keystore-ed25519")]
    fn ed25519_sign(
        &self,
        public: &ed25519::Public,
        msg: &[u8],
    ) -> Result<Option<ed25519::Signature>, crate::keystore::Error> {
        let lock = self.ed25519.read();
        let secret = lock.get(&Ed25519PublicWrapper(*public));
        if let Some(secret) = secret {
            Ok(Some(ed25519::sign(secret, msg)))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "keystore-ecdsa")]
    fn ecdsa_generate_new(
        &self,
        seed: Option<&[u8]>,
    ) -> Result<ecdsa::Public, crate::keystore::Error> {
        let secret = ecdsa::generate_with_optional_seed(seed)?;
        let public = secret.public_key();
        let old = self.ecdsa.write().insert(public, secret);
        assert!(old.is_none(), "generated key already exists");
        Ok(public)
    }

    #[cfg(feature = "keystore-ecdsa")]
    fn ecdsa_sign(
        &self,
        public: &ecdsa::Public,
        msg: &[u8],
    ) -> Result<Option<ecdsa::Signature>, crate::keystore::Error> {
        let lock = self.ecdsa.read();
        let secret = lock.get(public);
        if let Some(secret) = secret {
            Ok(Some(ecdsa::sign(secret, msg)))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "keystore-bls381")]
    fn bls381_generate_new(
        &self,
        seed: Option<&[u8]>,
    ) -> Result<bls381::Public, crate::keystore::Error> {
        let secret = bls381::generate_with_optional_seed(seed);
        let public = bls381::to_public(&secret);
        let old = self
            .bls381
            .write()
            .insert(Bls381PublicWrapper(public), secret);
        assert!(old.is_none(), "generated key already exists");
        Ok(public)
    }

    #[cfg(feature = "keystore-bls381")]
    fn bls381_sign(
        &self,
        public: &bls381::Public,
        msg: &[u8],
    ) -> Result<Option<bls381::Signature>, crate::keystore::Error> {
        let mut lock = self.bls381.write();
        let secret = lock.get_mut(&Bls381PublicWrapper(*public));
        if let Some(secret) = secret {
            Ok(Some(bls381::sign(secret, msg)))
        } else {
            Ok(None)
        }
    }

    #[cfg(feature = "keystore-sr25519")]
    fn expose_sr25519_secret(
        &self,
        public: &sr25519::Public,
    ) -> Result<Option<sr25519::Secret>, crate::keystore::Error> {
        let lock = self.sr25519.read();
        Ok(lock.get(public).cloned())
    }

    #[cfg(feature = "keystore-ecdsa")]
    fn expose_ecdsa_secret(
        &self,
        public: &ecdsa::Public,
    ) -> Result<Option<ecdsa::Secret>, crate::keystore::Error> {
        let lock = self.ecdsa.read();
        Ok(lock.get(public).cloned())
    }

    #[cfg(feature = "keystore-ed25519")]
    fn expose_ed25519_secret(
        &self,
        public: &ed25519::Public,
    ) -> Result<Option<ed25519::Secret>, crate::keystore::Error> {
        let lock = self.ed25519.read();
        Ok(lock.get(&Ed25519PublicWrapper(*public)).copied())
    }

    #[cfg(feature = "keystore-bls381")]
    fn expose_bls381_secret(
        &self,
        public: &bls381::Public,
    ) -> Result<Option<bls381::Secret>, crate::keystore::Error> {
        let lock = self.bls381.read();
        Ok(lock.get(&Bls381PublicWrapper(*public)).cloned())
    }

    #[cfg(feature = "keystore-sr25519")]
    fn iter_sr25519(&self) -> impl Iterator<Item = sr25519::Public> {
        let lock = self.sr25519.read();
        let iter = lock.keys().copied().collect::<Vec<_>>();
        iter.into_iter()
    }

    #[cfg(feature = "keystore-ecdsa")]
    fn iter_ecdsa(&self) -> impl Iterator<Item = ecdsa::Public> {
        let lock = self.ecdsa.read();
        let iter = lock.keys().copied().collect::<Vec<_>>();
        iter.into_iter()
    }

    #[cfg(feature = "keystore-ed25519")]
    fn iter_ed25519(&self) -> impl Iterator<Item = ed25519::Public> {
        let lock = self.ed25519.read();
        let iter = lock.keys().map(|k| k.0).collect::<Vec<_>>();
        iter.into_iter()
    }

    #[cfg(feature = "keystore-bls381")]
    fn iter_bls381(&self) -> impl Iterator<Item = bls381::Public> {
        let lock = self.bls381.read();
        let iter = lock.keys().map(|k| k.0).collect::<Vec<_>>();
        iter.into_iter()
    }
}
