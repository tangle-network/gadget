//! BLS12-381 keys and signatures.

use w3f_bls::TinyBLS381;

use crate::random;

/// BLS12-381 public key.
pub type Public = w3f_bls::PublicKey<TinyBLS381>;
/// BLS12-381 secret key.
pub type Secret = w3f_bls::SecretKey<TinyBLS381>;
/// BLS12-381 signature.
pub type Signature = w3f_bls::Signature<TinyBLS381>;

/// Generate a new secret key.
///
/// Note: If a seed is not provided, one will be generated using [`randon::getrandom_or_panic`].
#[must_use]
pub fn generate_with_optional_seed(seed: Option<&[u8]>) -> Secret {
    match seed {
        Some(seed) => Secret::from_seed(seed),
        None => Secret::generate(&mut random::getrandom_or_panic()),
    }
}

/// Sign a message with the given secret key.
pub fn sign(secret: &mut Secret, msg: &[u8]) -> Signature {
    let message = w3f_bls::Message::from(msg);
    secret.sign(&message, &mut random::getrandom_or_panic())
}

/// Derive the public key from the given secret key.
#[must_use]
pub fn to_public(secret: &Secret) -> Public {
    secret.clone().into_public()
}

/// Create a secret key from a byte slice.
///
/// This is equivalent to [`generate_with_optional_seed`] with a seed provided.
#[must_use]
pub fn secret_from_bytes(bytes: &[u8]) -> Secret {
    Secret::from_seed(bytes)
}
