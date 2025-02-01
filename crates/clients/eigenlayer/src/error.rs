use gadget_std::string::ParseError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] gadget_std::io::Error),
    #[error("Parse error {0}")]
    ParseError(#[from] ParseError),
    #[error("Url parse error {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Alloy contract error {0}")]
    AlloyContractError(#[from] alloy_contract::Error),
    #[error("Avs registry error: {0}")]
    AvsRegistryError(#[from] eigensdk::client_avsregistry::error::AvsRegistryError),
    #[error("El contracts error: {0}")]
    ElContractsError(#[from] eigensdk::client_elcontracts::error::ElContractsError),
    #[error("Operator service info error: {0}")]
    OperatorServiceInfoError(
        #[from] eigensdk::services_operatorsinfo::operatorsinfo_inmemory::OperatorInfoServiceError,
    ),
    #[error("Transport error: {0}")]
    TransportError(#[from] alloy_transport::RpcError<alloy_transport::TransportErrorKind>),
    #[error("Config error: {0}")]
    Config(#[from] gadget_config::Error),
    #[error("{0}")]
    OtherStatic(&'static str),
}

impl From<&'static str> for Error {
    fn from(e: &'static str) -> Self {
        Error::OtherStatic(e)
    }
}

impl From<Error> for gadget_client_core::error::Error {
    fn from(value: Error) -> Self {
        gadget_client_core::error::Error::Eigenlayer(value.to_string())
    }
}

pub type Result<T> = gadget_std::result::Result<T, Error>;
