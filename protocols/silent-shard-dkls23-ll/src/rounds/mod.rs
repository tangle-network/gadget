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
pub enum Error {
    #[error("i/o error")]
    IoError(#[source] IoError),
    #[error("unknown error")]
    SerializationError,
    #[error("DKLS23 keygen error")]
    DKLS23KeygenError(#[source] dkls23_ll::dkg::KeygenError),
    #[error("DKLS23 signing error")]
    DKLS23SigningError(#[source] dkls23_ll::dsg::SignError),
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Self::IoError(e)
    }
}

impl From<dkls23_ll::dkg::KeygenError> for Error {
    fn from(e: dkls23_ll::dkg::KeygenError) -> Self {
        Self::DKLS23KeygenError(e)
    }
}

impl From<dkls23_ll::dsg::SignError> for Error {
    fn from(e: dkls23_ll::dsg::SignError) -> Self {
        Self::DKLS23SigningError(e)
    }
}
