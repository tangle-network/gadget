use frost_core::Ciphersuite;
use round_based_21::rounds_router::{
    errors::{self as router_error, CompleteRoundError},
    simple_store::RoundInputError,
};
use std::convert::Infallible;
use thiserror::Error;

pub mod keygen;
pub mod sign;

pub type BoxedError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Error)]
pub enum IoError {
    #[error("send message")]
    SendMessage(#[source] BoxedError),
    #[error("receive message")]
    ReceiveMessage(#[source] BoxedError),
    #[error("got eof while recieving messages")]
    ReceiveMessageEof,
    #[error("route received message (possibly malicious behavior)")]
    RouteReceivedError(router_error::CompleteRoundError<RoundInputError, Infallible>),
}

impl IoError {
    pub fn send_message<E: std::error::Error + Send + Sync + 'static>(err: E) -> Self {
        Self::SendMessage(Box::new(err))
    }

    pub fn receive_message<E: std::error::Error + Send + Sync + 'static>(
        err: CompleteRoundError<RoundInputError, E>,
    ) -> Self {
        match err {
            CompleteRoundError::Io(router_error::IoError::Io(e)) => {
                Self::ReceiveMessage(Box::new(e))
            }
            CompleteRoundError::Io(router_error::IoError::UnexpectedEof) => Self::ReceiveMessageEof,

            CompleteRoundError::ProcessMessage(e) => {
                Self::RouteReceivedError(CompleteRoundError::ProcessMessage(e))
            }
            CompleteRoundError::Other(e) => Self::RouteReceivedError(CompleteRoundError::Other(e)),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error<C: Ciphersuite> {
    #[error("i/o error")]
    IoError(#[source] IoError),
    #[error("unknown error")]
    SerializationError,
    #[error("Frost keygen error")]
    FrostError(#[source] frost_core::Error<C>),
    #[error("Invalid frost protocol")]
    InvalidFrostProtocol,
}

impl<C: Ciphersuite> From<IoError> for Error<C> {
    fn from(e: IoError) -> Self {
        Self::IoError(e)
    }
}

impl<C: Ciphersuite> From<frost_core::Error<C>> for Error<C> {
    fn from(e: frost_core::Error<C>) -> Self {
        Self::FrostError(e)
    }
}
