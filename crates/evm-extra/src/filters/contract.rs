use alloy_primitives::Address;
use blueprint_core::JobCall;
use blueprint_core::error::BoxError;
use tower::filter::Predicate;

/// A [`Predicate`] that checks if the event came from a specific contract address.
#[derive(Debug, Clone, Copy)]
pub struct MatchesContract(pub Address);

/// An error that occurs during contract matching.
#[derive(Debug, Clone, Copy)]
pub enum MatchesContractError {
    /// No Logs in the Job call extensions.
    MissingLogs,
    /// The Logs in the Job call extensions are empty.
    EmptyLogs,
    /// No Log found in the Job call extensions that matches the expected contract address.
    NotFound(Address),
}

impl core::error::Error for MatchesContractError {}

impl core::fmt::Display for MatchesContractError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MatchesContractError::MissingLogs => write!(f, "No logs found in job call"),
            MatchesContractError::EmptyLogs => write!(f, "Logs in job call are empty"),
            MatchesContractError::NotFound(address) => write!(
                f,
                "No log found in job call that matches the expected contract address {address}",
            ),
        }
    }
}

impl Predicate<JobCall> for MatchesContract {
    type Request = JobCall;

    fn check(&mut self, call: Self::Request) -> Result<Self::Request, BoxError> {
        let (parts, body) = JobCall::into_parts(call);
        let logs = parts
            .extensions
            .get::<Vec<alloy_rpc_types::Log>>()
            .ok_or_else(|| Box::new(MatchesContractError::MissingLogs))?;

        if logs.is_empty() {
            return Err(Box::new(MatchesContractError::EmptyLogs));
        }
        for log in logs {
            if log.address() == self.0 {
                return Ok(JobCall::from_parts(parts, body));
            }
        }
        // No log found in the job call that matches the expected contract address
        Err(Box::new(MatchesContractError::NotFound(self.0)))
    }
}
