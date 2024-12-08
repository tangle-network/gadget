use crate::backend::{Error, KeyType};
use gadget_std::UniformRand;
use w3f_bls::{Message, PublicKey, SecretKey, Signature, TinyBLS381};

/// BLS381 key type
pub struct W3fBls381;

pub const CONTEXT: &[u8] = b"tangle";

impl KeyType for W3fBls381 {
    type Public = PublicKey<TinyBLS381>;
    type Secret = SecretKey<TinyBLS381>;
    type Signature = Signature<TinyBLS381>;

    fn generate_with_seed(seed: Option<&[u8]>) -> Result<Self::Secret, Error> {
        if let Some(seed) = seed {
            Ok(SecretKey::from_seed(seed))
        } else {
            // Should only be used for testing. Pass a seed in production.
            let mut rng = gadget_std::test_rng();
            let rand_bytes = <[u8; 32]>::rand(&mut rng);
            Ok(SecretKey::from_seed(&rand_bytes))
        }
    }

    fn public_from_secret(secret: &Self::Secret) -> Self::Public {
        secret.into_public()
    }

    fn sign_with_secret(secret: &mut Self::Secret, msg: &[u8]) -> Result<Self::Signature, Error> {
        let mut rng = Self::get_rng();
        let message: Message = Message::new(CONTEXT, msg);
        Ok(secret.sign(&message, &mut rng))
    }

    fn sign_with_secret_pre_hashed(
        secret: &mut Self::Secret,
        msg: &[u8; 32],
    ) -> Result<Self::Signature, Error> {
        let mut rng = Self::get_rng();
        let message: Message = Message::new(CONTEXT, msg);
        Ok(secret.sign(&message, &mut rng))
    }

    fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
        let message = Message::new(CONTEXT, msg);
        signature.verify(&message, public)
    }
}
