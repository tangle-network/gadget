use gadget_core::job::JobError;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    Keystore(String),
    Signature(String),
    Job(JobError),
    InvalidKeygenPartyId,
    InvalidSigningSet,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

impl From<JobError> for Error {
    fn from(value: JobError) -> Self {
        Error::Job(value)
    }
}

impl From<Error> for JobError {
    fn from(val: Error) -> Self {
        JobError {
            reason: val.to_string(),
        }
    }
}
