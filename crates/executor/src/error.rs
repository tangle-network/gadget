use std::{fmt, io};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Process exited unexpectedly with status: {0:?}")]
    UnexpectedExit(Option<i32>),
    #[error("Process with PID {0} not found in system")]
    ProcessNotFound(sysinfo::Pid),
    #[error("Service '{0}' not found in process manager")]
    ServiceNotFound(String),
    #[error("Process mismatch - Expected service '{0}' but found '{1}' - termination aborted")]
    ProcessMismatch(String, String),
    #[error("Failed to kill process (errno: {0}): {1}")]
    KillFailed(nix::errno::Errno, String),
    #[error("Output stream error for process {0}: {1}")]
    StreamError(sysinfo::Pid, String),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Formatting error: {0}")]
    Fmt(#[from] fmt::Error),
    #[error("JSON serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Process startup error: {0}")]
    StartupError(String),
    #[error("State recovery error: {0}")]
    StateRecoveryError(String),
    #[error("Read error: {0}")]
    ReadError(String),
    #[error("Invalid Command error: {0}")]
    InvalidCommand(String),
}
