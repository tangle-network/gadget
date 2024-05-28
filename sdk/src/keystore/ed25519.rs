//! Ed25519 Support using [`ed25519-zebra`] in pure Rust.

pub use ed25519_zebra::Signature;
pub use ed25519_zebra::SigningKey as Secret;
pub use ed25519_zebra::VerificationKey as Public;

use crate::random;

pub fn generate_with_optional_seed(seed: Option<&[u8]>) -> Result<Secret, ed25519_zebra::Error> {
    match seed {
        Some(seed) => Secret::try_from(seed).map_err(|_| ed25519_zebra::Error::InvalidSliceLength),
        None => Ok(Secret::new(&mut random::getrandom_or_panic())),
    }
}

pub fn sign(secret: &Secret, msg: &[u8]) -> Signature {
    secret.sign(msg)
}

pub fn to_public(secret: &Secret) -> Public {
    Public::from(secret)
}

pub fn secret_from_bytes(bytes: &[u8]) -> Result<Secret, ed25519_zebra::Error> {
    Secret::try_from(bytes).map_err(|_| ed25519_zebra::Error::InvalidSliceLength)
}
