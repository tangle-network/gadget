use crate::backend::{Error, KeyType};
use gadget_std::UniformRand;

/// Schnorrkel key type
pub struct SchnorrkelEcdsa;

impl KeyType for SchnorrkelEcdsa {
    type Public = schnorrkel::PublicKey;
    type Secret = schnorrkel::SecretKey;
    type Signature = schnorrkel::Signature;

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret, Error> {
        if let Some(seed) = seed {
            schnorrkel::SecretKey::from_bytes(seed).map_err(|e| Error::InvalidSeed(e.to_string()))
        } else {
            let mut rng = Self::get_rng();
            let rand_bytes: [u8; 32] = <[u8; 32]>::rand(&mut rng);
            schnorrkel::SecretKey::from_bytes(&rand_bytes)
                .map_err(|e| Error::InvalidSeed(e.to_string()))
        }
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        secret.to_public()
    }

    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature, Error> {
        let ctx = schnorrkel::signing_context(b"tangle").bytes(msg);
        Ok(secret.sign(ctx, &secret.to_public()))
    }

    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature, Error> {
        let ctx = schnorrkel::signing_context(b"tangle").bytes(msg);
        Ok(secret.sign(ctx, &secret.to_public()))
    }

    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
        let ctx = schnorrkel::signing_context(b"tangle").bytes(msg);
        if public.verify(ctx, signature).is_ok() {
            true
        } else {
            false
        }
    }
}
