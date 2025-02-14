use crate::extensions::Extensions;
use crate::metadata::{MetadataMap, MetadataValue};

#[derive(Clone, Debug)]
pub struct JobCall<T> {
    head: Parts,
    body: T,
}

impl<T: Default> Default for JobCall<T> {
    fn default() -> Self {
        Self {
            head: Parts::default(),
            body: T::default(),
        }
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Parts {
    /// The Job ID
    pub job_id: u32,
    /// Any metadata that were included in the job call
    pub metadata: MetadataMap<MetadataValue>,
    /// The job call extensions
    pub extensions: Extensions,
}

impl Parts {
    pub fn new(job_id: u32) -> Self {
        Self {
            job_id,
            metadata: MetadataMap::new(),
            extensions: Extensions::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: MetadataMap<MetadataValue>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = extensions;
        self
    }
}

impl Default for Parts {
    fn default() -> Self {
        Self {
            job_id: 0,
            metadata: MetadataMap::new(),
            extensions: Extensions::new(),
        }
    }
}

impl<T> JobCall<T> {
    pub fn empty(job_id: u32) -> Self
    where
        T: Default,
    {
        Self {
            head: Parts::new(job_id),
            body: Default::default(),
        }
    }
    pub fn new(job_id: u32, body: T) -> Self {
        Self {
            head: Parts::new(job_id),
            body,
        }
    }

    pub fn from_parts(parts: Parts, body: T) -> Self {
        Self { head: parts, body }
    }

    pub fn job_id(&self) -> u32 {
        self.head.job_id
    }

    pub fn job_id_mut(&mut self) -> &mut u32 {
        &mut self.head.job_id
    }

    pub fn metadata(&self) -> &MetadataMap<MetadataValue> {
        &self.head.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut MetadataMap<MetadataValue> {
        &mut self.head.metadata
    }

    pub fn extensions(&self) -> &Extensions {
        &self.head.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.head.extensions
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

    pub fn map<F, U>(self, f: F) -> JobCall<U>
    where
        F: FnOnce(T) -> U,
    {
        JobCall {
            head: self.head,
            body: f(self.body),
        }
    }
}
