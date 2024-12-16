use thiserror::Error;

pub mod contracts;

#[derive(Debug, Error)]
pub enum EvmEventListenerError {
    #[error("Client error: {0}")]
    Client(String),
    #[error("Transport error: {0}")]
    TransportError(#[from] alloy_transport::RpcError<alloy_transport::TransportErrorKind>),
}

pub type Result<T> = gadget_std::result::Result<T, EvmEventListenerError>;
