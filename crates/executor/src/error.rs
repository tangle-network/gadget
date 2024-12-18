use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("A process exited unexpectedly")]
    UnexpectedExit,
    #[error("Process {0} doesn't exist")]
    ProcessNotFound(sysinfo::Pid),
    #[error("Failed to focus on {0}, it does not exist")]
    ServiceNotFound(String),
    #[error("Expected {0} and found {1} running instead - process termination aborted")]
    ProcessMismatch(String, String),
    #[error("Failed to kill process, errno: {0}")]
    KillFailed(nix::errno::Errno),
    #[error("Output stream error for {0}")]
    StreamError(sysinfo::Pid),
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),
}
