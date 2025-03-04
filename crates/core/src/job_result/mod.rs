use crate::error::Error;
use crate::metadata::{MetadataMap, MetadataValue};

mod into_job_result;
mod into_job_result_parts;

pub use into_job_result::IntoJobResult;
pub use into_job_result_parts::IntoJobResultParts;
pub use into_job_result_parts::JobResultParts;

// TODO: More docs on this
/// A special result type that indicates a job produced no result
///
/// This is **not** the same as returning `None` or `()` from your [`Job`].
pub struct Void;

#[derive(Debug, Clone)]
pub enum JobResult<T, E = Error> {
    Ok { head: Parts, body: T },
    Err(E),
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Parts {
    /// Any Metadata that were included in this result.
    pub metadata: MetadataMap<MetadataValue>,
}

impl Parts {
    pub fn new() -> Self {
        Self {
            metadata: MetadataMap::new(),
        }
    }
}

impl<T, E> JobResult<T, E> {
    pub fn empty() -> Self
    where
        T: Default,
    {
        Self::Ok {
            head: Parts::new(),
            body: Default::default(),
        }
    }

    pub fn new(body: T) -> Self {
        Self::Ok {
            head: Parts::new(),
            body,
        }
    }

    pub fn from_parts(parts: Parts, body: T) -> Self {
        Self::Ok { head: parts, body }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, JobResult::Ok { .. })
    }

    pub fn is_err(&self) -> bool {
        matches!(self, JobResult::Err { .. })
    }

    pub fn metadata(&self) -> Option<&MetadataMap<MetadataValue>> {
        match self {
            JobResult::Ok { head, .. } => Some(&head.metadata),
            JobResult::Err(_) => None,
        }
    }

    pub fn metadata_mut(&mut self) -> Option<&mut MetadataMap<MetadataValue>> {
        match self {
            JobResult::Ok { head, .. } => Some(&mut head.metadata),
            JobResult::Err(_) => None,
        }
    }

    pub fn body_mut(&mut self) -> Result<&mut T, &E> {
        match self {
            JobResult::Ok { body, .. } => Ok(body),
            JobResult::Err(e) => Err(e),
        }
    }

    pub fn body(&self) -> Result<(&Parts, &T), &E> {
        match self {
            JobResult::Ok { head, body } => Ok((head, body)),
            JobResult::Err(err) => Err(err),
        }
    }

    pub fn into_parts(self) -> Result<(Parts, T), E> {
        match self {
            JobResult::Ok { head, body } => Ok((head, body)),
            JobResult::Err(err) => Err(err),
        }
    }

    pub fn map<F, U>(self, f: F) -> JobResult<U, E>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            JobResult::Ok { head, body } => JobResult::Ok {
                head,
                body: f(body),
            },
            JobResult::Err(err) => JobResult::Err(err),
        }
    }

    pub fn map_err<F, U>(self, f: F) -> JobResult<T, U>
    where
        F: FnOnce(E) -> U,
    {
        match self {
            JobResult::Ok { head, body } => JobResult::Ok { head, body },
            JobResult::Err(err) => JobResult::Err(f(err)),
        }
    }
}
