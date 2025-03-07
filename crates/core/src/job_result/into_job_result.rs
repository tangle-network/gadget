use super::{IntoJobResultParts, JobResultParts, Void};
use crate::JobResult;
use crate::error::{BoxError, Error};
use crate::job_result::Parts;
use crate::metadata::{MetadataMap, MetadataValue};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::{borrow::Cow, vec::Vec};
use bytes::{Bytes, BytesMut};
use core::convert::Infallible;

/// Trait for generating JobResults.
///
/// Types that implement `IntoJobResult` can be returned from [`Job`]s.
///
/// # Optional return value
///
/// The [`into_job_result`] method returns `Option<JobResult>`, as certain values can be considered
/// "void", meaning that they don't produce a result. This is useful in the case of [`Job`]s that
/// multiple parties are running, but only one party should submit the result. In this case, the
/// other parties should return [`Void`] from their [`Job`]s.
///
/// # Results
///
/// A special case exists for [`Result`], allowing you to return any [`Error`] from a [`Job`], so
/// long as it implements [`Send`] and [`Sync`].
///
/// ```rust
/// use blueprint_sdk::Router;
/// use blueprint_sdk::job::JobWithoutContextExt;
/// use blueprint_sdk::{IntoJobResult, JobResult};
///
/// #[derive(Debug)]
/// enum MyError {
///     SomethingWentWrong,
///     SomethingElseWentWrong,
/// }
///
/// impl core::fmt::Display for MyError {
///     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
///         match self {
///             MyError::SomethingWentWrong => write!(f, "something went wrong"),
///             MyError::SomethingElseWentWrong => write!(f, "something else went wrong"),
///         }
///     }
/// }
///
/// impl core::error::Error for MyError {}
///
/// const JOB_ID: u32 = 0;
///
/// // `Result<impl IntoJobResult, MyError>` can now be returned from jobs
/// let app = Router::new().route(JOB_ID, job);
///
/// async fn job() -> Result<(), MyError> {
///     Err(MyError::SomethingWentWrong)
/// }
/// # let _: Router = app;
/// ```
///
/// # Implementing `IntoJobResult`
///
/// You generally shouldn't have to implement `IntoJobResult` manually, as `blueprint_sdk`
/// provides implementations for many common types.
///
/// However, it might be necessary if you have a custom body type you want to return from a [`Job`]:
///
/// ```rust
/// use blueprint_sdk::job::JobWithoutContextExt;
/// use blueprint_sdk::{IntoJobResult, JobResult, Router};
/// use bytes::Bytes;
/// use std::{
///     convert::Infallible,
///     pin::Pin,
///     task::{Context, Poll},
/// };
///
/// struct SendHello;
///
/// impl SendHello {
///     fn hello_bytes(self) -> Bytes {
///         Bytes::from("hello")
///     }
/// }
///
/// // Now we can implement `IntoJobResult` directly for `MyBody`
/// impl IntoJobResult for SendHello {
///     fn into_job_result(self) -> Option<JobResult> {
///         Some(JobResult::new(self.hello_bytes()))
///     }
/// }
///
/// const JOB_ID: u32 = 0;
///
/// // `MyBody` can now be returned from jobs.
/// let app = Router::new().route(JOB_ID, || async { SendHello });
/// # let _: Router = app;
/// ```
///
/// [`Job`]: crate::job::Job
pub trait IntoJobResult {
    /// Create a JobResult.
    #[must_use]
    fn into_job_result(self) -> Option<JobResult>;
}

impl IntoJobResult for Void {
    fn into_job_result(self) -> Option<JobResult> {
        None
    }
}

impl IntoJobResult for () {
    fn into_job_result(self) -> Option<JobResult> {
        Bytes::new().into_job_result()
    }
}

impl IntoJobResult for Infallible {
    fn into_job_result(self) -> Option<JobResult> {
        match self {}
    }
}

// TODO: Is this possible to remove? Ideally, `Void` is the only way to return `None` from a handler.
impl<T> IntoJobResult for Option<T>
where
    T: IntoJobResult,
{
    fn into_job_result(self) -> Option<JobResult> {
        self.and_then(IntoJobResult::into_job_result)
    }
}

impl<T, E> IntoJobResult for Result<T, E>
where
    T: IntoJobResult,
    E: Into<BoxError>,
{
    fn into_job_result(self) -> Option<JobResult> {
        match self {
            Ok(value) => value.into_job_result(),
            Err(err) => Some(JobResult::Err(Error::new(err))),
        }
    }
}

impl<B> IntoJobResult for crate::job_result::JobResult<B>
where
    B: Into<Bytes> + Send + 'static,
{
    fn into_job_result(self) -> Option<JobResult> {
        Some(self.map(Into::into))
    }
}

impl IntoJobResult for Parts {
    fn into_job_result(self) -> Option<JobResult> {
        Some(JobResult::from_parts(self, Bytes::new()))
    }
}

impl IntoJobResult for Bytes {
    fn into_job_result(self) -> Option<JobResult> {
        Some(JobResult::new(self))
    }
}

impl IntoJobResult for &'static str {
    fn into_job_result(self) -> Option<JobResult> {
        Cow::Borrowed(self).into_job_result()
    }
}

impl IntoJobResult for String {
    fn into_job_result(self) -> Option<JobResult> {
        Cow::<'static, str>::Owned(self).into_job_result()
    }
}

impl IntoJobResult for Box<str> {
    fn into_job_result(self) -> Option<JobResult> {
        String::from(self).into_job_result()
    }
}

impl IntoJobResult for Cow<'static, str> {
    fn into_job_result(self) -> Option<JobResult> {
        Bytes::from(self.into_owned()).into_job_result()
    }
}

impl IntoJobResult for BytesMut {
    fn into_job_result(self) -> Option<JobResult> {
        self.freeze().into_job_result()
    }
}

impl IntoJobResult for &'static [u8] {
    fn into_job_result(self) -> Option<JobResult> {
        Cow::Borrowed(self).into_job_result()
    }
}

impl<const N: usize> IntoJobResult for &'static [u8; N] {
    fn into_job_result(self) -> Option<JobResult> {
        self.as_slice().into_job_result()
    }
}

impl<const N: usize> IntoJobResult for [u8; N] {
    fn into_job_result(self) -> Option<JobResult> {
        self.to_vec().into_job_result()
    }
}

impl IntoJobResult for Vec<u8> {
    fn into_job_result(self) -> Option<JobResult> {
        Cow::<'static, [u8]>::Owned(self).into_job_result()
    }
}

impl IntoJobResult for Box<[u8]> {
    fn into_job_result(self) -> Option<JobResult> {
        Vec::from(self).into_job_result()
    }
}

impl IntoJobResult for Cow<'static, [u8]> {
    fn into_job_result(self) -> Option<JobResult> {
        Bytes::from(self.into_owned()).into_job_result()
    }
}

impl IntoJobResult for MetadataMap<MetadataValue> {
    fn into_job_result(self) -> Option<JobResult> {
        let mut res = ().into_job_result()?;
        if let Some(metadata) = res.metadata_mut() {
            *metadata = self;
        }

        Some(res)
    }
}

impl<K, V, const N: usize> IntoJobResult for [(K, V); N]
where
    K: TryInto<&'static str>,
    K::Error: core::error::Error + Send + Sync + Into<BoxError> + 'static,
    V: TryInto<MetadataValue>,
    V::Error: core::error::Error + Send + Sync + Into<BoxError> + 'static,
{
    fn into_job_result(self) -> Option<JobResult> {
        (self, ()).into_job_result()
    }
}

impl<R> IntoJobResult for (Parts, R)
where
    R: IntoJobResult,
{
    fn into_job_result(self) -> Option<JobResult> {
        let (parts, res) = self;
        let mut org = res.into_job_result()?;
        if let Some(metadata) = org.metadata_mut() {
            metadata.extend(parts.metadata);
        }

        Some(org)
    }
}

impl<R> IntoJobResult for (crate::job_result::JobResult<()>, R)
where
    R: IntoJobResult,
{
    fn into_job_result(self) -> Option<JobResult> {
        let (template, res) = self;
        match template.into_parts() {
            Ok((parts, ())) => (parts, res).into_job_result(),
            Err(e) => Some(JobResult::Err(e)),
        }
    }
}

impl<R> IntoJobResult for (R,)
where
    R: IntoJobResult,
{
    fn into_job_result(self) -> Option<JobResult> {
        let (res,) = self;
        res.into_job_result()
    }
}

macro_rules! impl_into_job_result {
    ( $($ty:ident),* $(,)? ) => {
        #[allow(non_snake_case)]
        impl<R, $($ty,)*> IntoJobResult for ($($ty),*, R)
        where
            $( $ty: IntoJobResultParts, )*
            R: IntoJobResult,
        {
            fn into_job_result(self) -> Option<JobResult> {
                let ($($ty),*, res) = self;

                let res = res.into_job_result()?;
                let parts = JobResultParts { res };

                $(
                    let parts = match $ty.into_job_result_parts(parts) {
                        Ok(parts) => parts,
                        Err(err) => {
                            return Some(JobResult::Err(Error::new(err)));
                        }
                    };
                )*

                Some(parts.res)
            }
        }


        #[allow(non_snake_case)]
        impl<R, $($ty,)*> IntoJobResult for (crate::job_result::Parts, $($ty),*, R)
        where
            $( $ty: IntoJobResultParts, )*
            R: IntoJobResult,
        {
            fn into_job_result(self) -> Option<JobResult> {
                let (outer_parts, $($ty),*, res) = self;

                let res = res.into_job_result()?;
                let parts = JobResultParts { res };
                $(
                    let parts = match $ty.into_job_result_parts(parts) {
                        Ok(parts) => parts,
                        Err(err) => {
                            return Some(JobResult::Err(Error::new(err)));
                        }
                    };
                )*

                (outer_parts, parts.res).into_job_result()
            }
        }

        #[allow(non_snake_case)]
        impl<R, $($ty,)*> IntoJobResult for (crate::job_result::JobResult<()>, $($ty),*, R)
        where
            $( $ty: IntoJobResultParts, )*
            R: IntoJobResult,
        {
            fn into_job_result(self) -> Option<JobResult> {
                let (template, $($ty),*, res) = self;
                match template.into_parts() {
                    Ok((parts, ())) => {
                        (parts, $($ty),*, res).into_job_result()
                    },
                    Err(err) => Some(JobResult::Err(Error::new(err)))
                }
            }
        }
    }
}

all_the_tuples_no_last_special_case!(impl_into_job_result);
