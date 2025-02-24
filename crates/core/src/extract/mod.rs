//! Types and traits for extracting data from requests.
//!
//! See [`axum::extract`] for more details.
//!
//! [`axum::extract`]: https://docs.rs/axum/0.8/axum/extract/index.html

use crate::JobCall;
use crate::job_call::Parts;
use crate::job_result::IntoJobResult;
use core::convert::Infallible;
use core::future::Future;

pub mod rejection;

mod from_ref;
mod job_call_parts;
mod option;
mod tuple;

pub use self::{
    from_ref::FromRef,
    option::{OptionalFromJobCall, OptionalFromJobCallParts},
    rejection::InvalidUtf8,
};

mod private {
    #[derive(Debug, Clone, Copy)]
    pub enum ViaParts {}

    #[derive(Debug, Clone, Copy)]
    pub enum ViaJobCall {}
}

/// Types that can be created from job calls.
///
/// Extractors that implement `FromJobCall` can consume the job call body and can thus only be run
/// once for handlers.
///
/// If your extractor doesn't need to consume the job call body then you should implement
/// [`FromJobCallParts`] and not [`FromJobCall`].
///
/// See [`axum::extract`] for more general docs about extractors.
///
/// [`axum::extract`]: https://docs.rs/axum/0.8/axum/extract/index.html
#[diagnostic::on_unimplemented(
    note = "Function argument is not a valid blueprint extractor. \nSee `https://docs.rs/axum/0.8/axum/extract/index.html` for details"
)]
pub trait FromJobCallParts<Ctx>: Sized {
    /// If the extractor fails it'll use this "rejection" type. A rejection is
    /// a kind of error that can be converted into a job result.
    type Rejection: IntoJobResult;

    /// Perform the extraction.
    fn from_job_call_parts(
        parts: &mut Parts,
        ctx: &Ctx,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send;
}

/// Types that can be created from job calls.
///
/// Extractors that implement `FromJobCall` can consume the job call body and can thus only be run
/// once for handlers.
///
/// If your extractor doesn't need to consume the job call body then you should implement
/// [`FromJobCallParts`] and not [`FromJobCall`].
///
/// See [`axum::extract`] for more general docs about extractors.
///
/// [`axum::extract`]: https://docs.rs/axum/0.8/axum/extract/index.html
#[diagnostic::on_unimplemented(
    note = "Function argument is not a valid blueprint extractor. \nSee `https://docs.rs/axum/0.8/axum/extract/index.html` for details"
)]
pub trait FromJobCall<Ctx, M = private::ViaJobCall>: Sized {
    /// If the extractor fails it'll use this "rejection" type. A rejection is
    /// a kind of error that can be converted into a job result.
    type Rejection: IntoJobResult;

    /// Perform the extraction.
    fn from_job_call(
        call: JobCall,
        ctx: &Ctx,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send;
}

impl<Ctx, T> FromJobCall<Ctx, private::ViaParts> for T
where
    Ctx: Send + Sync,
    T: FromJobCallParts<Ctx>,
{
    type Rejection = <Self as FromJobCallParts<Ctx>>::Rejection;

    async fn from_job_call(call: JobCall, ctx: &Ctx) -> Result<Self, Self::Rejection> {
        let (mut parts, _) = call.into_parts();
        Self::from_job_call_parts(&mut parts, ctx).await
    }
}

impl<Ctx, T> FromJobCallParts<Ctx> for Result<T, T::Rejection>
where
    T: FromJobCallParts<Ctx>,
    Ctx: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_job_call_parts(parts: &mut Parts, ctx: &Ctx) -> Result<Self, Self::Rejection> {
        Ok(T::from_job_call_parts(parts, ctx).await)
    }
}

impl<Ctx, T> FromJobCall<Ctx> for Result<T, T::Rejection>
where
    T: FromJobCall<Ctx>,
    Ctx: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_job_call(call: JobCall, ctx: &Ctx) -> Result<Self, Self::Rejection> {
        Ok(T::from_job_call(call, ctx).await)
    }
}
