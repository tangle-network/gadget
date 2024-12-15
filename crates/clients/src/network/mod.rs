pub mod round_based_p2p;

#[cfg(feature = "eigenlayer")]
pub mod eigenlayer;

#[cfg(feature = "tangle")]
pub mod tangle;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to get party index: {0}")]
    PartyIndexError(String),

    #[error("Failed to get participants: {0}")]
    ParticipantsError(String),

    #[error("Failed to get blueprint ID: {0}")]
    BlueprintIdError(String),

    #[error("Failed to get party index and operators mapping: {0}")]
    PartyIndexOperatorsError(String),

    #[error("Failed to get service operator ECDSA keys: {0}")]
    ServiceOperatorKeysError(String),

    #[error("Failed to get current call ID: {0}")]
    CallIdError(String),

    #[error("Failed to create network delivery wrapper: {0}")]
    NetworkDeliveryError(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
