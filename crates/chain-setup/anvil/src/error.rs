#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error occurred in Testnet: {0}")]
    Container(String),
    #[error("Failed to mine anvil blocks: {0}")]
    Mine(String),
    #[error("Error occurred while waiting for responses: {0}")]
    WaitResponse(String),
    #[error("Contract Error: {0}")]
    Contract(#[from] alloy_contract::Error),
    #[error("Transaction Error: {0}")]
    Transaction(#[from] alloy_provider::PendingTransactionError),
    #[error("Keystore error: {0}")]
    Keystore(#[from] Box<gadget_keystore::Error>),
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(e: tokio::time::error::Elapsed) -> Self {
        Error::WaitResponse(e.to_string())
    }
}

impl From<gadget_keystore::Error> for Error {
    fn from(e: gadget_keystore::Error) -> Self {
        Box::new(e).into()
    }
}
