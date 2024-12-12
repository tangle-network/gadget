pub mod runtime;
pub mod services;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Subxt(#[from] subxt::Error),
    #[error("Client error: {0}")]
    Client(String),
    #[error("Other: {0}")]
    Other(String),
}
