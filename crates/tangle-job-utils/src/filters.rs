use blueprint_job_router::{BoxError, JobCall};
use tower::filter::Predicate;

use super::extract::ServiceId;

/// A [`Predicate`] that checks if the service ID matches the given in the job call.
#[derive(Debug, Clone, Copy)]
pub struct MatchesServiceId(pub u64);

#[derive(Debug)]
pub struct MismatchedServiceId {
    pub expected: u64,
    pub actual: u64,
}

impl core::fmt::Display for MismatchedServiceId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Mismatched service ID: expected {}, actual {}",
            self.expected, self.actual
        )
    }
}

impl core::error::Error for MismatchedServiceId {}

impl Predicate<JobCall> for MatchesServiceId {
    type Request = JobCall;
    fn check(&mut self, call: Self::Request) -> Result<Self::Request, BoxError> {
        let (mut parts, body) = JobCall::into_parts(call);
        let service_id = ServiceId::try_from(&mut parts)?;
        if service_id.0 == self.0 {
            Ok(JobCall::from_parts(parts, body))
        } else {
            Err(Box::new(MismatchedServiceId {
                expected: self.0,
                actual: service_id.0,
            }))
        }
    }
}
