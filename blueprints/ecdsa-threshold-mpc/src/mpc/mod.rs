use std::collections::HashMap;

use cggmp21::security_level::SecurityLevel128;
use cggmp21::{key_share::InvalidKeyShare, progress::ProfileError, security_level::SecurityLevel};
use generic_ec::Curve;
use sha2::Sha256;
use sp_core::ecdsa;

pub mod keygen;
pub mod refresh;
pub mod sign;

pub type DefaultSecurityLevel = SecurityLevel128;
pub type DefaultCryptoHasher = Sha256;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error")]
    Cggmp21IoError(String),
    #[error("SDK error")]
    SdkError(#[from] gadget_sdk::Error),
    #[error("Serialization error")]
    SerializationError(#[from] serde_json::Error),
    #[error("Keygen error")]
    KeygenError(#[from] cggmp21::keygen::KeygenError),
    #[error("Sign error")]
    SignError(#[from] cggmp21::signing::SigningError),
    #[error("Key refresh error")]
    KeyRefreshError(#[from] cggmp21::key_refresh::KeyRefreshError),
    #[error("Invalid key share")]
    ValidateError(String),
    #[error("Tokio join error")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("Tracer profile error")]
    TracerProfileError(#[from] cggmp21::progress::ProfileError),
}

// impl From<serde_json::Error> for Error {
//     fn from(e: serde_json::Error) -> Self {
//         Error::SerializationError(e)
//     }
// }

// impl From<ProfileError> for Error {
//     fn from(e: ProfileError) -> Self {
//         Error::TracerProfileError(e)
//     }
// }
