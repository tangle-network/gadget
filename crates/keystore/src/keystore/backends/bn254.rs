use super::*;
use crate::error::Result;
use gadget_crypto::bn254_crypto::{ArkBlsBn254Public, ArkBlsBn254Secret, ArkBlsBn254Signature};
use gadget_std::string::String;

#[async_trait::async_trait]
pub trait Bn254Backend: Send + Sync {
    /// Generate a new BN254 key pair from seed
    fn bls_bn254_generate_new(&self, seed: Option<&[u8]>) -> Result<ArkBlsBn254Public>;

    /// Generate a BN254 key pair from a string seed
    fn bls_bn254_generate_from_string(&self, secret: String) -> Result<ArkBlsBn254Public>;

    /// Sign a message using BN254 key
    fn bls_bn254_sign(
        &self,
        public: &ArkBlsBn254Public,
        msg: &[u8; 32],
    ) -> Result<Option<ArkBlsBn254Signature>>;

    /// Get the secret key for a BN254 public key
    fn expose_bls_bn254_secret(
        &self,
        public: &ArkBlsBn254Public,
    ) -> Result<Option<ArkBlsBn254Secret>>;

    /// Iterate over all BN254 public keys
    fn iter_bls_bn254(&self) -> Box<dyn Iterator<Item = ArkBlsBn254Public> + '_>;
}
