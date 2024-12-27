pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("No fetchers found for blueprint")]
    NoFetchers,
    #[error("Multiple fetchers found for blueprint")]
    MultipleFetchers,
    #[error("No testing fetcher found for blueprint, despite operating in test mode")]
    NoTestFetcher,
    #[error("Blueprint does not contain a supported fetcher")]
    UnsupportedGadget,

    #[error("Unable to find matching binary")]
    NoMatchingBinary,
    #[error("Binary hash {expected} mismatched expected hash of {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("Failed to build binary: {0:?}")]
    BuildBinary(std::process::Output),
    #[error("Failed to fetch git root: {0:?}")]
    FetchGitRoot(std::process::Output),

    #[error("Failed to get initial block hash")]
    InitialBlock,
    #[error("Finality Notification stream died")]
    ClientDied,
    #[error("{0}")]
    Other(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    TangleClient(#[from] gadget_clients::tangle::error::Error),
}
