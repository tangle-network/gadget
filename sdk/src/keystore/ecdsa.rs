//! ECDSA Support using [`k256`] in pure Rust.

use k256::ecdsa::signature::SignerMut;

pub use k256::ecdsa::Signature;
pub use k256::PublicKey as Public;
pub use k256::SecretKey as Secret;

use crate::random;

/// Generate a new secret key.
///
/// # Errors
///
/// Returns an error if the secret key cannot be created from `seed`.
pub fn generate_with_optional_seed(seed: Option<&[u8]>) -> elliptic_curve::Result<Secret> {
    if let Some(seed) = seed {
        Secret::from_bytes(seed.into())
    } else {
        let mut rng = random::getrandom_or_panic();
        Ok(Secret::random(&mut rng))
    }
}

/// Sign a message with the given secret key.
#[must_use]
pub fn sign(secret: &Secret, msg: &[u8]) -> Signature {
    let mut keypair = k256::ecdsa::SigningKey::from(secret);
    keypair.sign(msg)
}

/// Create a secret key from a byte slice.
///
/// # Errors
///
/// Returns an error if the secret key cannot be created from `bytes`.
pub fn secret_from_bytes(bytes: &[u8]) -> elliptic_curve::Result<Secret> {
    Secret::from_bytes(bytes.into())
}
