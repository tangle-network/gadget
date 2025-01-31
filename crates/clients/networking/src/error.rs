use gadget_std::string::String;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("P2P error: {0}")]
    P2p(String),
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Configuration error: {0}")]
    Configuration(String),
}

impl From<Error> for gadget_client_core::error::Error {
    fn from(value: Error) -> Self {
        gadget_client_core::error::Error::Network(value.to_string())
    }
}

pub type Result<T> = gadget_std::result::Result<T, Error>;
