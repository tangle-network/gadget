use tangle_subxt::subxt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TangleEventListenerError {
    #[error("Subxt error: {0}")]
    SubxtError(#[from] subxt::Error),
    #[error("Skip preprocessed type")]
    SkipPreProcessedType,
    #[error("Blueprint serde error: {0}")]
    BlueprintSerde(#[from] gadget_blueprint_serde::error::Error),
    #[error("Client error: {0}")]
    Client(String),
}

pub type Result<T> =
    gadget_std::result::Result<T, gadget_event_listeners_core::Error<TangleEventListenerError>>;
