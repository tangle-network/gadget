pub type Result<T> = gadget_std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Initializable event handler error: {0}")]
    EventHandler(String),

    #[error("The type has been skipped in the preprocessor")]
    SkipPreProcessedType,

    #[error("Bad argument decoding for {0}")]
    BadArgumentDecoding(String),

    #[error("Event loop ended unexpectedly")]
    Termination,

    #[error("{0}")]
    Other(String),
}
