use gadget_runners::core::error::RunnerError;
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
pub enum TestRunnerError {
    Io(std::io::Error),
    Serialization(serde_json::Error),
    Runner(RunnerError),
    Setup(String),
    Task(tokio::task::JoinError),
    Directory(String),
}

impl fmt::Display for TestRunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestRunnerError::Io(err) => write!(f, "IO error: {}", err),
            TestRunnerError::Serialization(err) => write!(f, "Serde JSON error: {}", err),
            TestRunnerError::Runner(err) => write!(f, "Runner error: {}", err),
            TestRunnerError::Setup(msg) => write!(f, "Setup error: {}", msg),
            TestRunnerError::Directory(msg) => write!(f, "Temporary directory error: {}", msg),
            TestRunnerError::Task(err) => write!(f, "Task error: {}", err),
        }
    }
}

impl StdError for TestRunnerError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            TestRunnerError::Io(err) => Some(err),
            TestRunnerError::Serialization(err) => Some(err),
            TestRunnerError::Runner(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TestRunnerError {
    fn from(err: std::io::Error) -> Self {
        TestRunnerError::Io(err)
    }
}

impl From<serde_json::Error> for TestRunnerError {
    fn from(err: serde_json::Error) -> Self {
        TestRunnerError::Serialization(err)
    }
}

impl From<RunnerError> for TestRunnerError {
    fn from(err: RunnerError) -> Self {
        TestRunnerError::Runner(err)
    }
}

impl From<tokio::task::JoinError> for TestRunnerError {
    fn from(err: tokio::task::JoinError) -> Self {
        TestRunnerError::Task(err)
    }
}
