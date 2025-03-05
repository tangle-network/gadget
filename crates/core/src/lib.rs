#![doc(
    html_logo_url = "https://cdn.prod.website-files.com/6494562b44a28080aafcbad4/65aaf8b0818b1d504cbdf81b_Tnt%20Logo.png"
)]
#![no_std]

extern crate alloc;

#[macro_use]
pub(crate) mod macros;

#[doc(hidden)]
pub mod __private {
    #[cfg(feature = "tracing")]
    pub use tracing;
}

pub mod error;
pub mod ext_traits;
pub mod extensions;
pub mod extract;
pub mod job;
pub mod job_call;
pub mod job_result;
pub mod metadata;

pub use self::error::Error;
pub use self::ext_traits::{job_call::JobCallExt, job_call_parts::JobCallPartsExt};
pub use self::extract::{FromJobCall, FromJobCallParts};
pub use self::job_call::job_id::{IntoJobId, JobId};
pub use self::job_result::{IntoJobResult, IntoJobResultParts};
pub use bytes::Bytes;
pub use job::Job;

/// A type representing a job result with a body of type `bytes::Bytes`.
pub type JobResult<T = Bytes> = crate::job_result::JobResult<T>;
/// A type representing a job call with a body of type `bytes::Bytes`.
pub type JobCall<T = Bytes> = crate::job_call::JobCall<T>;

// Feature-gated tracing macros, used by the entire SDK
macro_rules! tracing_macros {
    ($d:tt $($name:ident),*) => {
        $(
            #[doc(hidden)]
            #[cfg(feature = "tracing")]
            pub use tracing::$name;

            #[doc(hidden)]
            #[cfg(not(feature = "tracing"))]
            #[macro_export]
            macro_rules! debug {
                ($d($d tt:tt)*) => {};
            }
        )*
    }
}

tracing_macros!($
    info,
    warn,
    error,
    debug,
    trace
);
