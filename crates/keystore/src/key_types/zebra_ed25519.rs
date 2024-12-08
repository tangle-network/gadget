use crate::backend::{Error, KeyType};

/// Ed25519 key type
pub struct Ed25519Zebra;

impl KeyType for Ed25519Zebra {
    type Public = ed25519_zebra::VerificationKey;
    type Secret = ed25519_zebra::SigningKey;
    type Signature = ed25519_zebra::Signature;

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret, Error> {
        if let Some(seed) = seed {
            let seed = <[u8; 32]>::try_from(seed)
                .map_err(|_| Error::InvalidSeed("Seed is not 32 bytes!".to_string()))?;
            Ok(ed25519_zebra::SigningKey::from(seed))
        } else {
            let mut rng = Self::get_rng();
            Ok(ed25519_zebra::SigningKey::new(&mut rng))
        }
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        secret.into()
    }

    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature, Error> {
        Ok(secret.sign(msg))
    }

    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature, Error> {
        Ok(secret.sign(msg))
    }

    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
        if public.verify(signature, msg).is_ok() {
            true
        } else {
            false
        }
    }
}
