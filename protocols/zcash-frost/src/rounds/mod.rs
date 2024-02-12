use frost_core::Ciphersuite;
use round_based::rounds_router::{
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

/// Error indicating that protocol was aborted by malicious party
#[derive(Debug, Error)]
enum KeygenAborted<C: Ciphersuite> {
    #[error("Frost keygen error")]
    FrostError {
        parties: Vec<u16>,
        error: frost_core::Error<C>,
    },
}

/// Sign protocol error
#[derive(Debug, Error)]
enum SignAborted<C: Ciphersuite> {
    #[error("Frost sign error")]
    FrostError {
        parties: Vec<u16>,
        error: frost_core::Error<C>,
    },
    #[error("Invalid frost protocol")]
    InvalidFrostProtocol,
}

/// Keygen protocol error
#[derive(Debug, Error)]
#[error("keygen protocol is failed to complete")]
pub struct KeygenError<C: Ciphersuite>(#[source] Reason<C>);

impl<C: Ciphersuite> From<frost_core::Error<C>> for KeygenError<C> {
    fn from(err: frost_core::Error<C>) -> Self {
        match err {
            frost_core::Error::<C>::InvalidProofOfKnowledge { culprit } => {
                let culprit_bytes: Vec<u8> = culprit.serialize().as_ref().to_vec();
                let culprit = u16::from_le_bytes([culprit_bytes[0], culprit_bytes[1]]);
                KeygenError(Reason::KeygenFailure(KeygenAborted::FrostError {
                    parties: vec![culprit],
                    error: err,
                }))
            }
            _ => KeygenError(Reason::KeygenFailure(KeygenAborted::FrostError {
                parties: vec![],
                error: err,
            })),
        }
    }
}

impl<C: Ciphersuite> From<IoError> for KeygenError<C> {
    fn from(err: IoError) -> Self {
        KeygenError(Reason::IoError(err))
    }
}

impl<C: Ciphersuite> From<KeygenAborted<C>> for KeygenError<C> {
    fn from(err: KeygenAborted<C>) -> Self {
        KeygenError(Reason::KeygenFailure(err))
    }
}

/// Sign protocol error
#[derive(Debug, Error)]
#[error("keygen protocol is failed to complete")]
pub struct SignError<C: Ciphersuite>(#[source] Reason<C>);

impl<C: Ciphersuite> From<frost_core::Error<C>> for SignError<C> {
    fn from(err: frost_core::Error<C>) -> Self {
        match err {
            frost_core::Error::<C>::InvalidSignatureShare { culprit } => {
                let culprit_bytes: Vec<u8> = culprit.serialize().as_ref().to_vec();
                let culprit = u16::from_le_bytes([culprit_bytes[0], culprit_bytes[1]]);
                SignError(Reason::SignFailure(SignAborted::FrostError {
                    parties: vec![culprit],
                    error: err,
                }))
            }
            _ => SignError(Reason::SignFailure(SignAborted::FrostError {
                parties: vec![],
                error: err,
            })),
        }
    }
}

impl<C: Ciphersuite> From<IoError> for SignError<C> {
    fn from(err: IoError) -> Self {
        SignError(Reason::IoError(err))
    }
}

impl<C: Ciphersuite> From<SignAborted<C>> for SignError<C> {
    fn from(err: SignAborted<C>) -> Self {
        SignError(Reason::SignFailure(err))
    }
}

#[derive(Debug, Error)]
enum Reason<C: Ciphersuite> {
    /// Keygen protocol was maliciously aborted by another party
    #[error("keygen protocol was aborted by malicious party")]
    KeygenFailure(
        #[source]
        #[from]
        KeygenAborted<C>,
    ),
    #[error("sign protocol was aborted by malicious party")]
    SignFailure(
        #[source]
        #[from]
        SignAborted<C>,
    ),
    #[error("i/o error")]
    IoError(#[source] IoError),
    #[error("unknown error")]
    SerializationError,
}
