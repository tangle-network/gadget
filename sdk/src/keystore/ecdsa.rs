//! ECDSA Support using [`k256`] in pure Rust.

use k256::ecdsa::signature::SignerMut;

pub use k256::ecdsa::Signature;
pub use k256::PublicKey as Public;
pub use k256::SecretKey as Secret;

use crate::random;

pub fn generate_with_optional_seed(seed: Option<&[u8]>) -> k256::elliptic_curve::Result<Secret> {
    if let Some(seed) = seed {
        Secret::from_bytes(seed.into())
    } else {
        let mut rng = random::getrandom_or_panic();
        Ok(Secret::random(&mut rng))
    }
}

pub fn sign(secret: &Secret, msg: &[u8]) -> Signature {
    let mut keypair = k256::ecdsa::SigningKey::from(secret);
    keypair.sign(msg)
}

pub fn secret_from_bytes(bytes: &[u8]) -> k256::elliptic_curve::Result<Secret> {
    Secret::from_bytes(bytes.into())
}
