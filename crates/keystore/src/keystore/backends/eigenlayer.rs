use super::*;
use bn254::Bn254Backend;
use eigensdk::crypto_bls::{BlsG1Point, BlsSignature, PrivateKey};

#[async_trait::async_trait]
pub trait EigenlayerBackend: Bn254Backend {
    /// Sign a message using BN254 key
    async fn bls_bn254_sign(
        &self,
        public: &BlsG1Point,
        msg: &[u8; 32],
    ) -> Result<Option<BlsSignature>>;

    /// Get the secret key for a BN254 public key
    async fn expose_bls_bn254_secret(&self, public: &BlsG1Point) -> Result<Option<PrivateKey>>;

    /// Iterate over all BN254 public keys
    async fn iter_bls_bn254(&self) -> Box<dyn Iterator<Item = BlsG1Point> + '_>;

    /// Generate a new BN254 keypair with an optional seed
    async fn bls_bn254_generate_with_optional_seed(
        &self,
        seed: Option<[u8; 32]>,
    ) -> Result<BlsG1Point>;
}
