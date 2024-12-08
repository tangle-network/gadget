use crate::backend::{Error, KeyType};
use gadget_std::UniformRand;
use sp_core::Pair;

/// Ecdsa key type
pub struct SpBls381;

impl KeyType for SpBls381 {
    type Public = sp_core::bls381::Public;
    type Secret = sp_core::bls381::Pair;
    type Signature = sp_core::bls381::Signature;

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret, Error> {
        if let Some(seed) = seed {
            Ok(Pair::from_seed_slice(seed).map_err(|e| Error::InvalidSeed(e.to_string()))?)
        } else {
            let mut rng = Self::get_rng();
            let rand_bytes: [u8; 32] = <[u8; 32]>::rand(&mut rng);
            Ok(Pair::from_seed(&rand_bytes))
        }
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        <sp_core::bls381::Pair>::public(secret)
    }

    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature, Error> {
        Ok(<sp_core::bls381::Pair>::sign(&secret, msg))
    }

    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature, Error> {
        Ok(<sp_core::bls381::Pair>::sign(&secret, msg))
    }

    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
        <sp_core::bls381::Pair as Pair>::verify(signature, msg, public)
    }
}
