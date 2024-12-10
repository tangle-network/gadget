use crate::{
    error::Error,
    key_types::{arkworks_bn254, KeyType, KeyTypeId},
    storage::RawStorage,
};
use serde::de::DeserializeOwned;

#[async_trait::async_trait]
pub trait Bn254Backend: Send + Sync {
    /// Generate a new BN254 key pair from seed
    fn bls_bn254_generate_new(&self, seed: Option<&[u8]>) -> Result<arkworks_bn254::Public, Error>;

    /// Generate a BN254 key pair from a string seed
    fn bls_bn254_generate_from_string(
        &self,
        secret: String,
    ) -> Result<arkworks_bn254::Public, Error>;

    /// Sign a message using BN254 key
    fn bls_bn254_sign(
        &self,
        public: &arkworks_bn254::Public,
        msg: &[u8; 32],
    ) -> Result<Option<arkworks_bn254::ArkBlsBn254Signature>, Error>;

    /// Get the secret key for a BN254 public key
    fn expose_bls_bn254_secret(
        &self,
        public: &arkworks_bn254::Public,
    ) -> Result<Option<arkworks_bn254::Secret>, Error>;

    /// Iterate over all BN254 public keys
    fn iter_bls_bn254(&self) -> Box<dyn Iterator<Item = arkworks_bn254::Public> + '_>;
}

#[cfg(feature = "eigenlayer")]
mod eigenlayer {
    use super::*;
    use eigensdk::crypto_bls::{BlsG1Point, BlsSignature, PrivateKey};

    /// BN254 public key.
    pub type Public = BlsG1Point;
    /// BN254 secret key.
    pub type Secret = PrivateKey;
    /// BN254 signature.
    pub type Signature = BlsSignature;

    #[async_trait::async_trait]
    pub trait EigenlayerBackend: Bn254Backend {
        /// Sign a message using BN254 key
        async fn bls_bn254_sign(
            &self,
            public: &Public,
            msg: &[u8; 32],
        ) -> Result<Option<Signature>, Error>;

        /// Get the secret key for a BN254 public key
        async fn expose_bls_bn254_secret(&self, public: &Public) -> Result<Option<Secret>, Error>;

        /// Iterate over all BN254 public keys
        async fn iter_bls_bn254(&self) -> Box<dyn Iterator<Item = Public> + '_>;

        /// Generate a new BN254 keypair with an optional seed
        async fn bls_bn254_generate_with_optional_seed(
            &self,
            seed: Option<[u8; 32]>,
        ) -> Result<Public, Error>;
    }
}
