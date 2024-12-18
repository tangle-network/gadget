use gadget_runner_core::error::RunnerError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TangleError {
    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Keystore error: {0}")]
    Keystore(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<TangleError> for RunnerError {
    fn from(err: TangleError) -> Self {
        RunnerError::Tangle(err.to_string())
    }
}
