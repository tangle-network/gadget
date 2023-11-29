use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    Keystore(String),
    Signature(String),
    InvalidKeygenPartyId,
    InvalidSigningSet
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}