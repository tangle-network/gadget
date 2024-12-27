use gadget_std::string::String;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Provider error: {0}")]
    Provider(String),
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    #[error("Transaction error: {0}")]
    Transaction(String),
    #[error("Contract error: {0}")]
    Contract(String),
    #[error("ABI error: {0}")]
    Abi(String),
}

pub type Result<T> = gadget_std::result::Result<T, Error>;
