use crate::error::RunnerError;

#[derive(Debug, thiserror::Error)]
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
    Keystore(#[from] gadget_keystore::Error),

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
