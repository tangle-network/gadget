//! Schnorrkel keypair implementation.

pub use schnorrkel::PublicKey as Public;
pub use schnorrkel::SecretKey as Secret;
pub use schnorrkel::Signature;

/// The context used for signing.
const SIGNING_CTX: &[u8] = b"substrate";

pub fn generate_with_optional_seed(
    seed: Option<&[u8]>,
) -> Result<Secret, schnorrkel::SignatureError> {
    if let Some(seed) = seed {
        Secret::from_bytes(seed)
    } else {
        let rng = crate::random::getrandom_or_panic();
        Ok(Secret::generate_with(rng))
    }
}

pub fn sign(secret: &Secret, msg: &[u8]) -> Result<Signature, schnorrkel::SignatureError> {
    let public = secret.to_public();
    secret.sign_simple_doublecheck(SIGNING_CTX, msg, &public)
}

pub fn secret_from_bytes(bytes: &[u8]) -> Result<Secret, schnorrkel::SignatureError> {
    if bytes.len() == 32 {
        // add a new random nonce to the secret key
        let mut final_bytes = [0u8; 64];
        final_bytes[..32].copy_from_slice(bytes);
        let mut rng = crate::random::getrandom_or_panic();
        rand::Rng::fill(&mut rng, &mut final_bytes[32..]);
        Secret::from_bytes(&final_bytes[..])
    } else {
        Secret::from_bytes(bytes)
    }
}
