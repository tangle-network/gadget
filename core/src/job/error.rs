use alloc::string::String;
use core::error::Error;
use core::fmt::Display;

#[derive(Debug)]
pub struct JobError {
    pub reason: String,
}

impl<T: Into<String>> From<T> for JobError {
    fn from(value: T) -> Self {
        Self {
            reason: value.into(),
        }
    }
}

impl Display for JobError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{reason}", reason = self.reason)
    }
}

impl Error for JobError {}
