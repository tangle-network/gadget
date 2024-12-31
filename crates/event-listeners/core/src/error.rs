use gadget_std::fmt::Display;

#[derive(Debug, thiserror::Error)]
pub enum Error<T>
where
    T: core::fmt::Debug + core::error::Error,
{
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

    #[error("{0}")]
    ProcessorError(T),
}

impl<T> From<T> for Error<T>
where
    T: core::fmt::Debug + core::error::Error,
{
    fn from(e: T) -> Self {
        Error::ProcessorError(e)
    }
}

#[derive(Debug)]
pub struct Unit();

impl Display for Unit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "")
    }
}

impl core::error::Error for Unit {}
