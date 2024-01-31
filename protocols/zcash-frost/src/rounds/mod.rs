use frost_core::{Ciphersuite, FieldError};
use frost_ed25519::Ed25519Sha512;
use frost_ed448::Ed448Shake256;
use frost_p256::P256Sha256;
use frost_p384::P384Sha384;
use frost_redjubjub::JubjubBlake2b512;
use frost_ristretto255::Ristretto255Sha512;
use frost_secp256k1::Secp256K1Sha256;
use frost_taproot::Secp256K1Taproot;
use thiserror::Error;

use self::errors::{BoxedError, IoError};

pub mod errors;
pub mod keygen;
pub mod sign;

/// Keygen protocol error
#[derive(Debug, Error)]
#[error("keygen protocol is failed to complete")]
pub struct KeygenError<C: Ciphersuite>(#[source] Reason<C>);

macro_rules! impl_keygen_error_from {
    ($ciphersuite:ty) => {
        impl From<KeygenAborted<$ciphersuite>> for KeygenError<$ciphersuite> {
            fn from(err: KeygenAborted<$ciphersuite>) -> Self {
                KeygenError(Reason::KeygenFailure(err))
            }
        }

        impl From<IoError> for KeygenError<$ciphersuite> {
            fn from(err: IoError) -> Self {
                KeygenError(Reason::IoError(err))
            }
        }
    };
}

impl_keygen_error_from!(Ed25519Sha512);
impl_keygen_error_from!(P256Sha256);
impl_keygen_error_from!(P384Sha384);
impl_keygen_error_from!(Ristretto255Sha512);
impl_keygen_error_from!(Secp256K1Sha256);
impl_keygen_error_from!(Ed448Shake256);
impl_keygen_error_from!(JubjubBlake2b512);
impl_keygen_error_from!(Secp256K1Taproot);

/// Sign protocol error
#[derive(Debug, Error)]
#[error("keygen protocol is failed to complete")]
pub struct SignError<C: Ciphersuite>(#[source] Reason<C>);

macro_rules! impl_sign_error_from {
    ($ciphersuite:ty) => {
        impl From<SignAborted<$ciphersuite>> for SignError<$ciphersuite> {
            fn from(err: SignAborted<$ciphersuite>) -> Self {
                SignError(Reason::SignFailure(err))
            }
        }

        impl From<IoError> for SignError<$ciphersuite> {
            fn from(err: IoError) -> Self {
                SignError(Reason::IoError(err))
            }
        }
    };
}

impl_sign_error_from!(Ed25519Sha512);
impl_sign_error_from!(P256Sha256);
impl_sign_error_from!(P384Sha384);
impl_sign_error_from!(Ristretto255Sha512);
impl_sign_error_from!(Secp256K1Sha256);
impl_sign_error_from!(Ed448Shake256);
impl_sign_error_from!(JubjubBlake2b512);
impl_sign_error_from!(Secp256K1Taproot);

#[derive(Debug, Error)]
enum Reason<C: Ciphersuite> {
    /// Keygen protocol was maliciously aborted by another party
    #[error("keygen protocol was aborted by malicious party")]
    KeygenFailure(
        #[source]
        #[from]
        KeygenAborted<C>,
    ),
    #[error("sign protocol was aborted by malicious party")]
    SignFailure(
        #[source]
        #[from]
        SignAborted<C>,
    ),
    #[error("field error")]
    FieldError(#[source] FieldError),
    #[error("i/o error")]
    IoError(#[source] IoError),
    #[error("unknown error")]
    SerializationError,
}

/// Error indicating that protocol was aborted by malicious party
///
/// It _can be_ cryptographically proven, but we do not support it yet.
#[derive(Debug, Error)]
enum KeygenAborted<C: Ciphersuite> {
    #[error("Frost keygen error")]
    FrostError {
        parties: Vec<u16>,
        error: frost_core::Error<C>,
    },
}

/// Sign protocol error
///
/// It _can be_ cryptographically proven, but we do not support it yet.
#[derive(Debug, Error)]
enum SignAborted<C: Ciphersuite> {
    #[error("Frost sign error")]
    FrostError {
        parties: Vec<u16>,
        error: frost_core::Error<C>,
    },
}
