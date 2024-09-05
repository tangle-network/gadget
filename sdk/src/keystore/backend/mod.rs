//! Keystore backend implementations.

use crate::keystore::bn254::{Public, Secret, Signature};
use crate::keystore::Error;
use alloc::vec::Vec;

/// In-Memory Keystore Backend
pub mod mem;

/// Filesystem Keystore Backend
#[cfg(feature = "std")]
pub mod fs;

/// A Generic Key Store that can be backed by different keystore backends.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum GenericKeyStore<RwLock: lock_api::RawRwLock> {
    /// In-Memory Keystore
    Mem(mem::InMemoryKeystore<RwLock>),
    /// Filesystem Keystore
    #[cfg(feature = "std")]
    Fs(fs::FilesystemKeystore),
}

impl<RwLock: lock_api::RawRwLock> super::Backend for GenericKeyStore<RwLock> {
    fn sr25519_generate_new(
        &self,
        seed: Option<&[u8]>,
    ) -> Result<super::sr25519::Public, super::Error> {
        match self {
            Self::Mem(backend) => backend.sr25519_generate_new(seed),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.sr25519_generate_new(seed),
        }
    }

    fn sr25519_sign(
        &self,
        public: &super::sr25519::Public,
        msg: &[u8],
    ) -> Result<Option<super::sr25519::Signature>, super::Error> {
        match self {
            Self::Mem(backend) => backend.sr25519_sign(public, msg),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.sr25519_sign(public, msg),
        }
    }

    fn ed25519_generate_new(
        &self,
        seed: Option<&[u8]>,
    ) -> Result<super::ed25519::Public, super::Error> {
        match self {
            Self::Mem(backend) => backend.ed25519_generate_new(seed),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.ed25519_generate_new(seed),
        }
    }

    fn ed25519_sign(
        &self,
        public: &super::ed25519::Public,
        msg: &[u8],
    ) -> Result<Option<super::ed25519::Signature>, super::Error> {
        match self {
            Self::Mem(backend) => backend.ed25519_sign(public, msg),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.ed25519_sign(public, msg),
        }
    }

    fn ecdsa_generate_new(
        &self,
        seed: Option<&[u8]>,
    ) -> Result<super::ecdsa::Public, super::Error> {
        match self {
            Self::Mem(backend) => backend.ecdsa_generate_new(seed),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.ecdsa_generate_new(seed),
        }
    }

    fn ecdsa_sign(
        &self,
        public: &super::ecdsa::Public,
        msg: &[u8],
    ) -> Result<Option<super::ecdsa::Signature>, super::Error> {
        match self {
            Self::Mem(backend) => backend.ecdsa_sign(public, msg),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.ecdsa_sign(public, msg),
        }
    }

    fn bls381_generate_new(
        &self,
        seed: Option<&[u8]>,
    ) -> Result<super::bls381::Public, super::Error> {
        match self {
            Self::Mem(backend) => backend.bls381_generate_new(seed),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.bls381_generate_new(seed),
        }
    }

    fn bls381_sign(
        &self,
        public: &super::bls381::Public,
        msg: &[u8],
    ) -> Result<Option<super::bls381::Signature>, super::Error> {
        match self {
            Self::Mem(backend) => backend.bls381_sign(public, msg),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.bls381_sign(public, msg),
        }
    }

    fn bls_bn254_generate_new(&self, seed: Option<&[u8]>) -> Result<Public, Error> {
        match self {
            Self::Mem(backend) => backend.bls_bn254_generate_new(seed),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.bls_bn254_generate_new(seed),
        }
    }

    fn bls_bn254_sign(&self, public: &Public, msg: &[u8]) -> Result<Option<Signature>, Error> {
        match self {
            Self::Mem(backend) => backend.bls_bn254_sign(public, msg),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.bls_bn254_sign(public, msg),
        }
    }

    fn expose_sr25519_secret(
        &self,
        public: &super::sr25519::Public,
    ) -> Result<Option<super::sr25519::Secret>, super::Error> {
        match self {
            Self::Mem(backend) => backend.expose_sr25519_secret(public),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.expose_sr25519_secret(public),
        }
    }

    fn expose_ecdsa_secret(
        &self,
        public: &super::ecdsa::Public,
    ) -> Result<Option<super::ecdsa::Secret>, super::Error> {
        match self {
            Self::Mem(backend) => backend.expose_ecdsa_secret(public),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.expose_ecdsa_secret(public),
        }
    }

    fn expose_ed25519_secret(
        &self,
        public: &super::ed25519::Public,
    ) -> Result<Option<super::ed25519::Secret>, super::Error> {
        match self {
            Self::Mem(backend) => backend.expose_ed25519_secret(public),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.expose_ed25519_secret(public),
        }
    }

    fn expose_bls381_secret(
        &self,
        public: &super::bls381::Public,
    ) -> Result<Option<super::bls381::Secret>, super::Error> {
        match self {
            Self::Mem(backend) => backend.expose_bls381_secret(public),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.expose_bls381_secret(public),
        }
    }

    fn expose_bls_bn254_secret(&self, public: &Public) -> Result<Option<Secret>, Error> {
        match self {
            Self::Mem(backend) => backend.expose_bls_bn254_secret(public),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.expose_bls_bn254_secret(public),
        }
    }

    fn iter_sr25519(&self) -> impl Iterator<Item = super::sr25519::Public> {
        match self {
            Self::Mem(backend) => backend.iter_sr25519().collect::<Vec<_>>().into_iter(),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.iter_sr25519().collect::<Vec<_>>().into_iter(),
        }
    }

    fn iter_ecdsa(&self) -> impl Iterator<Item = super::ecdsa::Public> {
        match self {
            Self::Mem(backend) => backend.iter_ecdsa().collect::<Vec<_>>().into_iter(),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.iter_ecdsa().collect::<Vec<_>>().into_iter(),
        }
    }

    fn iter_ed25519(&self) -> impl Iterator<Item = super::ed25519::Public> {
        match self {
            Self::Mem(backend) => backend.iter_ed25519().collect::<Vec<_>>().into_iter(),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.iter_ed25519().collect::<Vec<_>>().into_iter(),
        }
    }

    fn iter_bls381(&self) -> impl Iterator<Item = super::bls381::Public> {
        match self {
            Self::Mem(backend) => backend.iter_bls381().collect::<Vec<_>>().into_iter(),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.iter_bls381().collect::<Vec<_>>().into_iter(),
        }
    }

    fn iter_bls_bn254(&self) -> impl Iterator<Item = Public> {
        match self {
            Self::Mem(backend) => backend.iter_bls_bn254().collect::<Vec<_>>().into_iter(),
            #[cfg(feature = "std")]
            Self::Fs(backend) => backend.iter_bls_bn254().collect::<Vec<_>>().into_iter(),
        }
    }
}
