use super::{IntoJobResultParts, JobResultParts};
use crate::JobResult;
use crate::job_result::Parts;
use crate::metadata::{MetadataMap, MetadataValue};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::{borrow::Cow, vec::Vec};
use bytes::{Bytes, BytesMut};
use core::{convert::Infallible, fmt};

/// Trait for generating JobResults.
///
/// Types that implement `IntoJobResult` can be returned from job handlers.
///
/// # Implementing `IntoJobResult`
///
/// You generally shouldn't have to implement `IntoJobResult` manually, as axum
/// provides implementations for many common types.
///
/// However it might be necessary if you have a custom error type that you want
/// to return from handlers:
///
/// ```rust
/// use blueprint_sdk::Router;
/// use blueprint_sdk::job::JobWithoutContextExt;
/// use blueprint_sdk::{IntoJobResult, JobResult};
///
/// enum MyError {
///     SomethingWentWrong,
///     SomethingElseWentWrong,
/// }
///
/// impl IntoJobResult for MyError {
///     fn into_job_result(self) -> JobResult {
///         let body = match self {
///             MyError::SomethingWentWrong => "something went wrong",
///             MyError::SomethingElseWentWrong => "something else went wrong",
///         };
///
///         body.into_job_result()
///     }
/// }
///
/// const HANDLER_JOB_ID: u32 = 0;
///
/// // `Result<impl IntoJobResult, MyError>` can now be returned from handlers
/// let app = Router::new().route(HANDLER_JOB_ID, handler);
///
/// async fn handler() -> Result<(), MyError> {
///     Err(MyError::SomethingWentWrong)
/// }
/// # let _: Router = app;
/// ```
///
/// Or if you have a custom body type you'll also need to implement
/// `IntoJobResult` for it:
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
///     fn into_job_result(self) -> JobResult {
///         JobResult::new(self.hello_bytes())
///     }
/// }
///
/// const JOB_ID: u32 = 0;
///
/// // `MyBody` can now be returned from handlers.
/// let app = Router::new().route(JOB_ID, (|| async { SendHello }).into_service());
/// # let _: Router = app;
/// ```
pub trait IntoJobResult {
    /// Create a JobResult.
    #[must_use]
    fn into_job_result(self) -> JobResult;
}

impl IntoJobResult for () {
    fn into_job_result(self) -> JobResult {
        Bytes::new().into_job_result()
    }
}

impl IntoJobResult for Infallible {
    fn into_job_result(self) -> JobResult {
        match self {}
    }
}

impl<T, E> IntoJobResult for Result<T, E>
where
    T: IntoJobResult,
    E: IntoJobResult,
{
    fn into_job_result(self) -> JobResult {
        match self {
            Ok(value) => value.into_job_result(),
            Err(err) => err.into_job_result(),
        }
    }
}

impl<B> IntoJobResult for crate::job_result::JobResult<B>
where
    B: Into<Bytes> + Send + 'static,
{
    fn into_job_result(self) -> JobResult {
        self.map(Into::into)
    }
}

impl IntoJobResult for Parts {
    fn into_job_result(self) -> JobResult {
        JobResult::from_parts(self, Bytes::new())
    }
}

impl IntoJobResult for Bytes {
    fn into_job_result(self) -> JobResult {
        JobResult::new(self)
    }
}

impl IntoJobResult for &'static str {
    fn into_job_result(self) -> JobResult {
        Cow::Borrowed(self).into_job_result()
    }
}

impl IntoJobResult for String {
    fn into_job_result(self) -> JobResult {
        Cow::<'static, str>::Owned(self).into_job_result()
    }
}

impl IntoJobResult for Box<str> {
    fn into_job_result(self) -> JobResult {
        String::from(self).into_job_result()
    }
}

impl IntoJobResult for Cow<'static, str> {
    fn into_job_result(self) -> JobResult {
        Bytes::from(self.into_owned()).into_job_result()
    }
}

impl IntoJobResult for BytesMut {
    fn into_job_result(self) -> JobResult {
        self.freeze().into_job_result()
    }
}

impl IntoJobResult for &'static [u8] {
    fn into_job_result(self) -> JobResult {
        Cow::Borrowed(self).into_job_result()
    }
}

impl<const N: usize> IntoJobResult for &'static [u8; N] {
    fn into_job_result(self) -> JobResult {
        self.as_slice().into_job_result()
    }
}

impl<const N: usize> IntoJobResult for [u8; N] {
    fn into_job_result(self) -> JobResult {
        self.to_vec().into_job_result()
    }
}

impl IntoJobResult for Vec<u8> {
    fn into_job_result(self) -> JobResult {
        Cow::<'static, [u8]>::Owned(self).into_job_result()
    }
}

impl IntoJobResult for Box<[u8]> {
    fn into_job_result(self) -> JobResult {
        Vec::from(self).into_job_result()
    }
}

impl IntoJobResult for Cow<'static, [u8]> {
    fn into_job_result(self) -> JobResult {
        Bytes::from(self.into_owned()).into_job_result()
    }
}

impl IntoJobResult for MetadataMap<MetadataValue> {
    fn into_job_result(self) -> JobResult {
        let mut res = ().into_job_result();
        *res.metadata_mut() = self;
        res
    }
}

impl<K, V, const N: usize> IntoJobResult for [(K, V); N]
where
    K: TryInto<&'static str>,
    K::Error: fmt::Display,
    V: TryInto<MetadataValue>,
    V::Error: fmt::Display,
{
    fn into_job_result(self) -> JobResult {
        (self, ()).into_job_result()
    }
}

impl<R> IntoJobResult for (Parts, R)
where
    R: IntoJobResult,
{
    fn into_job_result(self) -> JobResult {
        let (parts, res) = self;
        let mut org = res.into_job_result();
        org.metadata_mut().extend(parts.metadata);
        org
    }
}

impl<R> IntoJobResult for (crate::job_result::JobResult<()>, R)
where
    R: IntoJobResult,
{
    fn into_job_result(self) -> JobResult {
        let (template, res) = self;
        let (parts, ()) = template.into_parts();
        (parts, res).into_job_result()
    }
}

impl<R> IntoJobResult for (R,)
where
    R: IntoJobResult,
{
    fn into_job_result(self) -> JobResult {
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
            fn into_job_result(self) -> JobResult {
                let ($($ty),*, res) = self;

                let res = res.into_job_result();
                let parts = JobResultParts { res };

                $(
                    let parts = match $ty.into_job_result_parts(parts) {
                        Ok(parts) => parts,
                        Err(err) => {
                            return err.into_job_result();
                        }
                    };
                )*

                parts.res
            }
        }


        #[allow(non_snake_case)]
        impl<R, $($ty,)*> IntoJobResult for (crate::job_result::Parts, $($ty),*, R)
        where
            $( $ty: IntoJobResultParts, )*
            R: IntoJobResult,
        {
            fn into_job_result(self) -> JobResult {
                let (outer_parts, $($ty),*, res) = self;

                let res = res.into_job_result();
                let parts = JobResultParts { res };
                $(
                    let parts = match $ty.into_job_result_parts(parts) {
                        Ok(parts) => parts,
                        Err(err) => {
                            return err.into_job_result();
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
            fn into_job_result(self) -> JobResult {
                let (template, $($ty),*, res) = self;
                let (parts, ()) = template.into_parts();
                (parts, $($ty),*, res).into_job_result()
            }
        }
    }
}

all_the_tuples_no_last_special_case!(impl_into_job_result);
