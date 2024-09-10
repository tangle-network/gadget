//! Schnorrkel keypair implementation.

pub use schnorrkel::PublicKey as Public;
pub use schnorrkel::SecretKey as Secret;
pub use schnorrkel::Signature;

/// The context used for signing.
const SIGNING_CTX: &[u8] = b"substrate";

/// Sign a message with the given secret key.
///
/// # Errors
///
/// Returns an error if the signature fails to be verified.
pub fn sign(secret: &Secret, msg: &[u8]) -> Result<Signature, schnorrkel::SignatureError> {
    let public = secret.to_public();
    secret.sign_simple_doublecheck(SIGNING_CTX, msg, &public)
}

/// Generate a new secret key.
///
/// # Errors
///
/// * `seed` is not the correct length (32 or 64).
/// * `seed` contains an invalid scalar.
pub fn generate_with_optional_seed(
    seed: Option<&[u8]>,
) -> Result<Secret, schnorrkel::SignatureError> {
    if let Some(seed) = seed {
        secret_from_bytes(seed)
    } else {
        let rng = crate::random::getrandom_or_panic();
        Ok(Secret::generate_with(rng))
    }
}

/// Create a secret key from a byte slice.
///
/// If the slice is 32 bytes long, a deterministic nonce is appended
///
/// # Errors
///
/// * `bytes` is not the correct length (32 or 64).
/// * `bytes` contains an invalid scalar.
pub fn secret_from_bytes(bytes: &[u8]) -> Result<Secret, schnorrkel::SignatureError> {
    if bytes.len() == 32 {
        let mini_secret = schnorrkel::MiniSecretKey::from_bytes(bytes)?;
        Ok(mini_secret.expand(schnorrkel::MiniSecretKey::ED25519_MODE))
    } else {
        Secret::from_bytes(bytes)
    }
}
