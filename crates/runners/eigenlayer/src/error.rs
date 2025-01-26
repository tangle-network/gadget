use eigensdk::{
    client_avsregistry::error::AvsRegistryError, client_elcontracts::error::ElContractsError,
};
use gadget_runner_core::error::RunnerError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EigenlayerError {
    #[error("AVS Registry error: {0}")]
    AvsRegistry(#[from] AvsRegistryError),

    #[error("Contract error: {0}")]
    Contract(#[from] alloy_contract::Error),

    #[error("EL Contracts error: {0}")]
    ElContracts(#[from] ElContractsError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("Registration error: {0}")]
    Registration(String),

    #[error("Task error: {0}")]
    Task(String),

    #[error("Keystore error: {0}")]
    Keystore(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<EigenlayerError> for RunnerError {
    fn from(err: EigenlayerError) -> Self {
        RunnerError::Eigenlayer(err.to_string())
    }
}
