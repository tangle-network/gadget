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
pub struct JobResult<T> {
    head: Parts,
    body: T,
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

impl<T> JobResult<T> {
    pub fn empty() -> Self
    where
        T: Default,
    {
        Self {
            head: Parts::new(),
            body: Default::default(),
        }
    }
    pub fn new(body: T) -> Self {
        Self {
            head: Parts::new(),
            body,
        }
    }

    pub fn from_parts(parts: Parts, body: T) -> Self {
        Self { head: parts, body }
    }

    pub fn metadata(&self) -> &MetadataMap<MetadataValue> {
        &self.head.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut MetadataMap<MetadataValue> {
        &mut self.head.metadata
    }

    pub fn body_mut(&mut self) -> &mut T {
        &mut self.body
    }

    pub fn body(&self) -> &T {
        &self.body
    }

    pub fn into_body(self) -> T {
        self.body
    }

    pub fn into_parts(self) -> (Parts, T) {
        (self.head, self.body)
    }

    pub fn map<F, U>(self, f: F) -> JobResult<U>
    where
        F: FnOnce(T) -> U,
    {
        JobResult {
            head: self.head,
            body: f(self.body),
        }
    }
}
