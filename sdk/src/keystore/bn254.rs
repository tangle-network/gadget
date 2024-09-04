//! BLS BN254 keys and signatures

use crate::random;
use ark_bn254::{Fq, Fr, G1Affine, G1Projective};
use elliptic_curve::bigint::Pow;
use k256::ecdsa::signature::SignerMut;

/// BN254 public key.
pub type Public = G1Projective;
/// BN254 secret key.
pub type Secret = Fr;
/// BN254 signature.
pub type Signature = G1Affine;

#[must_use]
pub fn generate_with_optional_seed(seed: Option<&[u8]>) -> Secret {
    match seed {
        Some(seed) => Secret::from_seed(seed),
        None => Secret::generate(&mut random::getrandom_or_panic()),
    }
}

pub fn sign(secret: &mut Secret, msg: &[u8]) -> Signature {
    let hashed_message = map_to_curve(msg.into());
    let sig = hashed_message.mul_bigint(secret.0);
    sig.into_affine()
}

pub fn map_to_curve(digest: &[u8; 32]) -> Public {
    let one = Fq::one();
    let three = Fq::from(3u64);
    let mut x = Fq::from_be_bytes_mod_order(digest.as_slice());

    loop {
        let x_cubed = x.pow([3]);
        let y = x_cubed + three;

        if y.legendre().is_qr() {
            let y = y.sqrt().unwrap();
            let point = G1Affine::new(x, y);

            if point.is_on_curve() {
                return G1Projective::new(point.x, point.y, F::one());
            }
        }

        x += one;
    }
}

#[must_use]
pub fn to_public(secret: &Secret) -> Public {
    secret.clone().into_public()
}

#[must_use]
pub fn secret_from_bytes(bytes: &[u8]) -> Secret {
    Secret::from_seed(bytes)
}
