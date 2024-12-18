use gadget_runner_core::error::RunnerError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SymbioticError {
    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Registration error: {0}")]
    Registration(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<SymbioticError> for RunnerError {
    fn from(err: SymbioticError) -> Self {
        RunnerError::Symbiotic(err.to_string())
    }
}

// Convenience type alias
pub type Result<T> = std::result::Result<T, SymbioticError>;
