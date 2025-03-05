use job_id::{IntoJobId, JobId};

use crate::extensions::Extensions;
use crate::metadata::{MetadataMap, MetadataValue};

/// Job Identifiers
pub mod job_id;

/// Representation of a job call event
///
/// These events come from [Producers], which may be listening for on-chain events
/// (e.g. [`TangleProducer`]), or simply waiting for a timer to end (e.g. [`CronJob`]).
///
/// A `JobCall` consists of two parts: a header and a body.
///
/// ## Header
///
/// The header is a [`Parts`] containing the [`JobId`] and any metadata associated with the call.
/// This metadata is used by argument extractors, see the [extract] module for more information.
///
/// ## Body
///
/// The body will typically contain the arguments associated with the job call, in a producer-specific format.
/// For example, the [`TangleProducer`] will produce `JobCall`s with a body compatible with
/// the [`TangleArg`] extractors.
///
/// [`CronJob`]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/producers/struct.CronJob.html
/// [`TangleProducer`]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/tangle/producer/struct.TangleProducer.html
/// [`TangleArg`]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/tangle/extract/struct.TangleArg.html
/// [Producers]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/producers/index.html
/// [extract]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/extract/index.html
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

/// The metadata associated with a [`JobCall`]
///
/// This metadata is used by argument extractors, see the [extract] module for more information.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Parts {
    /// The Job ID
    pub job_id: JobId,
    /// Any metadata that were included in the job call
    pub metadata: MetadataMap<MetadataValue>,
    /// The job call extensions
    pub extensions: Extensions,
}

impl Parts {
    /// Create a new `Parts` with the given `job_id`
    pub fn new<J: IntoJobId>(job_id: J) -> Self {
        Self {
            job_id: job_id.into_job_id(),
            metadata: MetadataMap::new(),
            extensions: Extensions::new(),
        }
    }

    /// Set the metadata to a pre-defined [`MetadataMap`]
    pub fn with_metadata(mut self, metadata: MetadataMap<MetadataValue>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set the extensions to a pre-defined [`Extensions`]
    pub fn with_extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = extensions;
        self
    }
}

impl Default for Parts {
    fn default() -> Self {
        Self {
            job_id: JobId::ZERO,
            metadata: MetadataMap::new(),
            extensions: Extensions::new(),
        }
    }
}

impl<T> JobCall<T> {
    /// Create an empty `JobCall`
    ///
    /// This is useful for producers such as [`CronJob`], where the intention is to simply trigger a
    /// job with no extra arguments.
    ///
    /// [`CronJob`]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/producers/struct.CronJob.html
    pub fn empty() -> Self
    where
        T: Default,
    {
        Self {
            head: Parts::new(JobId::ZERO),
            body: Default::default(),
        }
    }

    /// Create a new `JobCall` with the given `job_id` and `body`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blueprint_sdk::{IntoJobId, JobCall};
    ///
    /// const MY_JOB_ID: u8 = 0;
    ///
    /// let call = JobCall::new(MY_JOB_ID, ());
    /// assert_eq!(call.job_id(), MY_JOB_ID.into_job_id());
    /// ```
    pub fn new<J: IntoJobId>(job_id: J, body: T) -> Self {
        Self {
            head: Parts::new(job_id),
            body,
        }
    }

    /// Create a new `JobCall` with a pre-defined `parts` and `body`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blueprint_sdk::job_call::Parts;
    /// use blueprint_sdk::{IntoJobId, JobCall};
    ///
    /// const MY_JOB_ID: u8 = 0;
    /// let parts = Parts::new(MY_JOB_ID);
    ///
    /// let call = JobCall::from_parts(parts, ());
    /// assert_eq!(call.job_id(), MY_JOB_ID.into_job_id());
    /// ```
    pub fn from_parts(parts: Parts, body: T) -> Self {
        Self { head: parts, body }
    }

    /// Get the job id of this `JobCall`
    pub fn job_id(&self) -> JobId {
        self.head.job_id
    }

    /// Get a mutable reference to the job id of this `JobCall`
    pub fn job_id_mut(&mut self) -> &mut JobId {
        &mut self.head.job_id
    }

    /// Get a reference to the call metadata
    pub fn metadata(&self) -> &MetadataMap<MetadataValue> {
        &self.head.metadata
    }

    /// Get a mutable reference to the call metadata
    pub fn metadata_mut(&mut self) -> &mut MetadataMap<MetadataValue> {
        &mut self.head.metadata
    }

    /// Get a reference to the call extensions
    pub fn extensions(&self) -> &Extensions {
        &self.head.extensions
    }

    /// Get a mutable reference to the call extensions
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.head.extensions
    }

    /// Get a reference to the body
    pub fn body(&self) -> &T {
        &self.body
    }

    /// Consume the `JobCall` and return the body
    pub fn into_body(self) -> T {
        self.body
    }

    pub fn into_parts(self) -> (Parts, T) {
        (self.head, self.body)
    }

    /// Takes a closure and applies it to the body
    ///
    /// # Examples
    ///
    /// ```rust
    /// use blueprint_sdk::job_call::JobCall;
    ///
    /// let call = JobCall::new(0, ());
    /// let new_call = call.map(|_| "Hello!");
    /// assert_eq!(*new_call.body(), "Hello!");
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
