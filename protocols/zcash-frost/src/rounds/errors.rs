use std::convert::Infallible;

use round_based::rounds_router::{
    errors::{self as router_error, CompleteRoundError},
    simple_store::RoundInputError,
};
use thiserror::Error;

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
