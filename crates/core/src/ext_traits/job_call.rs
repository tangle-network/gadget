use crate::JobCall;
use crate::extract::{FromJobCall, FromJobCallParts};
use core::future::Future;

mod sealed {
    use crate::JobCall;

    pub trait Sealed {}
    impl Sealed for JobCall {}
}

/// Extension trait that adds additional methods to [`JobCall`].
pub trait JobCallExt: sealed::Sealed + Sized {
    /// Apply an extractor to this `JobCall`.
    ///
    /// This is just a convenience for `E::from_job_call(call, &())`.
    ///
    /// Note this consumes the job call. Use [`JobCallExt::extract_parts`] if you're not extracting
    /// the body and don't want to consume the job call.
    fn extract<E, M>(self) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCall<(), M> + 'static,
        M: 'static;

    /// Apply an extractor that requires some context to this `JobCall`.
    ///
    /// This is just a convenience for `E::from_job_call(call, ctx)`.
    ///
    /// Note this consumes the job call. Use [`JobCallExt::extract_parts_with_state`] if you're not
    /// extracting the body and don't want to consume the job call.
    ///
    /// # Example
    ///
    /// ```
    /// use blueprint_sdk::extract::{FromJobCall, FromRef};
    /// use blueprint_sdk::{JobCall, JobCallExt};
    ///
    /// struct MyExtractor {
    ///     requires_context: RequiresContext,
    /// }
    ///
    /// impl<Ctx> FromJobCall<Ctx> for MyExtractor
    /// where
    ///     String: FromRef<Ctx>,
    ///     Ctx: Send + Sync,
    /// {
    ///     type Rejection = std::convert::Infallible;
    ///
    ///     async fn from_job_call(call: JobCall, context: &Ctx) -> Result<Self, Self::Rejection> {
    ///         let requires_context = call.extract_with_ctx::<RequiresContext, _, _>(context).await?;
    ///
    ///         Ok(Self { requires_context })
    ///     }
    /// }
    ///
    /// // some extractor that consumes the call body and requires a context
    /// struct RequiresContext { /* ... */ }
    ///
    /// impl<Ctx> FromJobCall<Ctx> for RequiresContext
    /// where
    ///     String: FromRef<Ctx>,
    ///     Ctx: Send + Sync,
    /// {
    ///     // ...
    ///     # type Rejection = std::convert::Infallible;
    ///     # async fn from_job_call(call: JobCall, _context: &Ctx) -> Result<Self, Self::Rejection> {
    ///     #     todo!()
    ///     # }
    /// }
    /// ```
    fn extract_with_ctx<E, Ctx, M>(
        self,
        ctx: &Ctx,
    ) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCall<Ctx, M> + 'static,
        Ctx: Send + Sync;

    /// Apply a parts extractor to this `JobCall`.
    ///
    /// This is just a convenience for `E::from_job_call_parts(parts, ctx)`.
    fn extract_parts<E>(&mut self) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCallParts<()> + 'static;

    /// Apply a parts extractor that requires some state to this `Request`.
    ///
    /// This is just a convenience for `E::from_job_call_parts(parts, ctx)`.
    fn extract_parts_with_ctx<'a, E, Ctx>(
        &'a mut self,
        ctx: &'a Ctx,
    ) -> impl Future<Output = Result<E, E::Rejection>> + Send + 'a
    where
        E: FromJobCallParts<Ctx> + 'static,
        Ctx: Send + Sync;
}

impl JobCallExt for JobCall {
    fn extract<E, M>(self) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCall<(), M> + 'static,
        M: 'static,
    {
        self.extract_with_ctx(&())
    }

    fn extract_with_ctx<E, Ctx, M>(
        self,
        ctx: &Ctx,
    ) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCall<Ctx, M> + 'static,
        Ctx: Send + Sync,
    {
        E::from_job_call(self, ctx)
    }

    fn extract_parts<E>(&mut self) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCallParts<()> + 'static,
    {
        self.extract_parts_with_ctx(&())
    }

    async fn extract_parts_with_ctx<'a, E, Ctx>(
        &'a mut self,
        ctx: &'a Ctx,
    ) -> Result<E, E::Rejection>
    where
        E: FromJobCallParts<Ctx> + 'static,
        Ctx: Send + Sync,
    {
        let mut call = crate::job_call::JobCall::default();
        *call.job_id_mut() = self.job_id();
        *call.metadata_mut() = core::mem::take(self.metadata_mut());
        let (mut parts, ()) = call.into_parts();

        let result = E::from_job_call_parts(&mut parts, ctx).await;

        *self.job_id_mut() = parts.job_id;
        *self.metadata_mut() = core::mem::take(&mut parts.metadata);

        result
    }
}
