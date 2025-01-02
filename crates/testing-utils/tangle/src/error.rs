use gadget_runners::core::error::RunnerError;
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Serialization(serde_json::Error),
    Runner(RunnerError),
    Setup(String),
    Task(tokio::task::JoinError),
    Directory(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "IO error: {}", err),
            Error::Serialization(err) => write!(f, "Serde JSON error: {}", err),
            Error::Runner(err) => write!(f, "Runner error: {}", err),
            Error::Setup(msg) => write!(f, "Setup error: {}", msg),
            Error::Directory(msg) => write!(f, "Temporary directory error: {}", msg),
            Error::Task(err) => write!(f, "Task error: {}", err),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::Serialization(err) => Some(err),
            Error::Runner(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err)
    }
}

impl From<RunnerError> for Error {
    fn from(err: RunnerError) -> Self {
        Error::Runner(err)
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(err: tokio::task::JoinError) -> Self {
        Error::Task(err)
    }
}
