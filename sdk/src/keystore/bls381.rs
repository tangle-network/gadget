//! BLS12-381 keys and signatures.

use w3f_bls::TinyBLS381;

/// BLS12-381 public key.
pub type Public = w3f_bls::PublicKey<TinyBLS381>;
/// BLS12-381 secret key.
pub type Secret = w3f_bls::SecretKey<TinyBLS381>;
/// BLS12-381 signature.
pub type Signature = w3f_bls::Signature<TinyBLS381>;
