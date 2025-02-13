#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
extern crate core;

/// Alias for a type-erased error type.
pub type BoxError = alloc::boxed::Box<dyn core::error::Error + Send + Sync>;

#[macro_use]
pub(crate) mod macros;

#[doc(hidden)]
pub mod __private {
    #[cfg(feature = "tracing")]
    pub use tracing;
}

mod boxed;
pub mod context;
mod error;
pub mod ext_traits;
pub mod extract;
pub mod job;
pub mod job_call;
pub mod job_result;
pub mod metadata;
pub mod routing;
pub mod service_ext;
#[cfg(test)]
pub(crate) mod test_helpers;
mod util;

pub use self::error::Error;
pub use self::ext_traits::{job_call::JobCallExt, job_call_parts::JobCallPartsExt};
pub use self::extract::{FromJobCall, FromJobCallParts};
pub use self::job_result::{IntoJobResult, IntoJobResultParts};
pub use bytes::Bytes;
pub use context::Context;
pub use job::Job;
pub use routing::Router;
pub use service_ext::ServiceExt;

/// A type representing a job result with a body of type `bytes::Bytes`.
pub type JobResult<T = Bytes> = crate::job_result::JobResult<T>;
/// A type representing a job call with a body of type `bytes::Bytes`.
pub type JobCall<T = Bytes> = crate::job_call::JobCall<T>;
