//! Ed25519 Support using [`ed25519-zebra`] in pure Rust.

pub use ed25519_zebra::Signature;
pub use ed25519_zebra::SigningKey as Secret;
pub use ed25519_zebra::VerificationKey as Public;

use crate::random;

/// Generate a new keypair.
///
/// # Errors
///
/// Returns an error if the `seed` is not the correct length (32).
pub fn generate_with_optional_seed(seed: Option<&[u8]>) -> Result<Secret, ed25519_zebra::Error> {
    match seed {
        Some(seed) => Secret::try_from(seed).map_err(|_| ed25519_zebra::Error::InvalidSliceLength),
        None => Ok(Secret::new(random::getrandom_or_panic())),
    }
}

/// Sign a message with the given secret key.
#[must_use]
pub fn sign(secret: &Secret, msg: &[u8]) -> Signature {
    secret.sign(msg)
}

/// Derive the public key from the given secret key.
#[must_use]
pub fn to_public(secret: &Secret) -> Public {
    Public::from(secret)
}

/// Create a signing key from a byte slice.
///
/// # Errors
///
/// Returns an error if the slice is not the correct length (32).
pub fn secret_from_bytes(bytes: &[u8]) -> Result<Secret, ed25519_zebra::Error> {
    Secret::try_from(bytes).map_err(|_| ed25519_zebra::Error::InvalidSliceLength)
}
