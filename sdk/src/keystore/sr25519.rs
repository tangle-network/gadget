//! Schnorrkel keypair implementation.

use rand::prelude::ThreadRng;
pub use schnorrkel::PublicKey as Public;
pub use schnorrkel::SecretKey as Secret;
pub use schnorrkel::Signature;

/// The context used for signing.
const SIGNING_CTX: &[u8] = b"substrate";

pub fn generate_with_optional_seed(
    seed: Option<&[u8]>,
) -> Result<Secret, schnorrkel::SignatureError> {
    match seed {
        Some(seed) => Secret::from_bytes(seed),
        None => Ok(schnorrkel::SecretKey::generate_with(&mut ThreadRng::default())),
    }
}

pub fn sign(secret: &Secret, msg: &[u8]) -> Result<Signature, schnorrkel::SignatureError> {
    let public = secret.to_public();
    secret.sign_simple_doublecheck(SIGNING_CTX, msg, &public)
}

pub fn secret_from_bytes(bytes: &[u8]) -> Result<Secret, schnorrkel::SignatureError> {
    Secret::from_bytes(bytes)
}
