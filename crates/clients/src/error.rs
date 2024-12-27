#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] gadget_client_core::Error),
    #[error(transparent)]
    #[cfg(feature = "eigenlayer")]
    Eigenlayer(#[from] gadget_client_eigenlayer::error::Error),
    #[error(transparent)]
    #[cfg(feature = "evm")]
    Evm(#[from] gadget_client_evm::error::Error),
    #[error(transparent)]
    #[cfg(feature = "networking")]
    Networking(#[from] gadget_client_networking::error::Error),
    #[error(transparent)]
    #[cfg(feature = "tangle")]
    Tangle(#[from] gadget_client_tangle::error::Error),
}
