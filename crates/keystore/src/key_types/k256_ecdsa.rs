use crate::backend::{Error, KeyType};
use gadget_std::UniformRand;
use k256::ecdsa::signature::SignerMut;

/// ECDSA key type
pub struct K256Ecdsa;

impl KeyType for K256Ecdsa {
    type Public = k256::ecdsa::VerifyingKey;
    type Secret = k256::ecdsa::SigningKey;
    type Signature = k256::ecdsa::Signature;

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret, Error> {
        if let Some(seed) = seed {
            k256::ecdsa::SigningKey::from_bytes(seed.into())
                .map_err(|e| Error::InvalidSeed(e.to_string()))
        } else {
            let mut rng = Self::get_rng();
            let rand_bytes: [u8; 32] = <[u8; 32]>::rand(&mut rng);
            k256::ecdsa::SigningKey::from_slice(&rand_bytes)
                .map_err(|e| Error::InvalidSeed(e.to_string()))
        }
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        *secret.verifying_key()
    }

    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature, Error> {
        Ok(secret.sign(msg))
    }

    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature, Error> {
        let signature = secret
            .sign_prehash_recoverable(msg)
            .map_err(|e| Error::SignatureFailed(e.to_string()))?;
        Ok(signature.0)
    }

    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
        use k256::ecdsa::signature::Verifier;
        public.verify(msg, signature).is_ok()
    }
}
