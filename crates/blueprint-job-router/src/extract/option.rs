use core::future::Future;

use crate::extract::{private, FromJobCall, FromJobCallParts, JobCall};
use crate::job_call::Parts;
use crate::job_result::IntoJobResult;

/// Customize the behavior of `Option<Self>` as a [`FromJobCallParts`]
/// extractor.
pub trait OptionalFromJobCallParts<Ctx>: Sized {
    /// If the extractor fails, it will use this "rejection" type.
    ///
    /// A rejection is a kind of error that can be converted into a job result.
    type Rejection: IntoJobResult;

    /// Perform the extraction.
    fn from_job_call_parts(
        parts: &mut Parts,
        ctx: &Ctx,
    ) -> impl Future<Output = Result<Option<Self>, Self::Rejection>> + Send;
}

/// Customize the behavior of `Option<Self>` as a [`FromJobCall`] extractor.
pub trait OptionalFromJobCall<Ctx, M = private::ViaJobCall>: Sized {
    /// If the extractor fails, it will use this "rejection" type.
    ///
    /// A rejection is a kind of error that can be converted into a job result..
    type Rejection: IntoJobResult;

    /// Perform the extraction.
    fn from_job_call(
        call: JobCall,
        ctx: &Ctx,
    ) -> impl Future<Output = Result<Option<Self>, Self::Rejection>> + Send;
}

impl<Ctx, T> FromJobCallParts<Ctx> for Option<T>
where
    T: OptionalFromJobCallParts<Ctx>,
    Ctx: Send + Sync,
{
    type Rejection = T::Rejection;

    fn from_job_call_parts(
        parts: &mut Parts,
        ctx: &Ctx,
    ) -> impl Future<Output = Result<Option<T>, Self::Rejection>> {
        T::from_job_call_parts(parts, ctx)
    }
}

impl<Ctx, T> FromJobCall<Ctx> for Option<T>
where
    T: OptionalFromJobCall<Ctx>,
    Ctx: Send + Sync,
{
    type Rejection = T::Rejection;

    async fn from_job_call(call: JobCall, ctx: &Ctx) -> Result<Option<T>, Self::Rejection> {
        T::from_job_call(call, ctx).await
    }
}
