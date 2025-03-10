use alloy_primitives::B256;
use blueprint_core::JobCall;
use blueprint_core::error::BoxError;
use tower::filter::Predicate;

/// A [`Predicate`] that checks if the event matches a specific event signature.
#[derive(Debug, Clone, Copy)]
pub struct MatchesEvent(pub B256);

/// An error that occurs during contract matching.
#[derive(Debug, Clone, Copy)]
pub enum MatchesEventError {
    /// No Logs in the Job call extensions.
    MissingLogs,
    /// The Logs in the Job call extensions are empty.
    EmptyLogs,
    /// No Log found in the Job call extensions that matches the expected event signature.
    NotFound(B256),
}

impl core::error::Error for MatchesEventError {}

impl core::fmt::Display for MatchesEventError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MatchesEventError::MissingLogs => write!(f, "No logs found in job call"),
            MatchesEventError::EmptyLogs => write!(f, "Logs in job call are empty"),
            MatchesEventError::NotFound(signature) => write!(
                f,
                "No log found in job call that matches the expected event signature {signature}",
            ),
        }
    }
}

impl Predicate<JobCall> for MatchesEvent {
    type Request = JobCall;

    fn check(&mut self, call: Self::Request) -> Result<Self::Request, BoxError> {
        let (parts, body) = JobCall::into_parts(call);
        let logs = parts
            .extensions
            .get::<Vec<alloy_rpc_types::Log>>()
            .ok_or_else(|| Box::new(MatchesEventError::MissingLogs))?;

        if logs.is_empty() {
            return Err(Box::new(MatchesEventError::EmptyLogs));
        }
        for log in logs {
            if log.topic0() == Some(&self.0) {
                return Ok(JobCall::from_parts(parts, body));
            }
        }
        // No log found in the job call that matches the expected contract address
        Err(Box::new(MatchesEventError::NotFound(self.0)))
    }
}
