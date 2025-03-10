use super::Job;

use crate::JobResult;
use crate::error::BoxError;
use crate::job_call::JobCall;
use bytes::Bytes;
use core::{
    fmt,
    marker::PhantomData,
    task::{Context, Poll},
};
use tower::Service;

/// An adapter that makes a [`Job`] into a [`Service`].
///
/// Created with [`Job::with_context`] or [`JobWithoutContextExt::into_service`].
///
/// [`JobWithoutContextExt::into_service`]: super::JobWithoutContextExt::into_service
pub struct JobService<J, T, Ctx> {
    job: J,
    ctx: Ctx,
    _marker: PhantomData<fn() -> T>,
}

impl<J, T, Ctx> JobService<J, T, Ctx> {
    /// Get a reference to the state.
    pub fn context(&self) -> &Ctx {
        &self.ctx
    }
}

#[test]
fn traits() {
    pub(crate) fn assert_send<T: Send>() {}
    pub(crate) fn assert_sync<T: Sync>() {}
    #[allow(dead_code)]
    pub(crate) struct NotSendSync(*const ());
    assert_send::<JobService<(), NotSendSync, ()>>();
    assert_sync::<JobService<(), NotSendSync, ()>>();
}

impl<J, T, Ctx> JobService<J, T, Ctx> {
    pub(super) fn new(job: J, ctx: Ctx) -> Self {
        Self {
            job,
            ctx,
            _marker: PhantomData,
        }
    }
}

impl<J, T, Ctx> fmt::Debug for JobService<J, T, Ctx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntoService").finish_non_exhaustive()
    }
}

impl<J, T, Ctx> Clone for JobService<J, T, Ctx>
where
    J: Clone,
    Ctx: Clone,
{
    fn clone(&self) -> Self {
        Self {
            job: self.job.clone(),
            ctx: self.ctx.clone(),
            _marker: PhantomData,
        }
    }
}

impl<J, T, Ctx, B> Service<JobCall<B>> for JobService<J, T, Ctx>
where
    J: Job<T, Ctx> + Clone + Send + 'static,
    B: Into<Bytes> + Send + 'static,
    Ctx: Clone + Send + Sync,
{
    type Response = Option<JobResult>;
    type Error = BoxError;
    type Future = super::future::IntoServiceFuture<J::Future>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // `IntoService` can only be constructed from async functions which are always ready, or
        // from `Layered` which buffers in `<Layered as Handler>::call` and is therefore
        // also always ready.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, call: JobCall<B>) -> Self::Future {
        use futures_util::future::FutureExt;

        let call = call.map(Into::into);

        let handler = self.job.clone();
        let future = Job::call(handler, call, self.ctx.clone());
        let future = future.map(Ok as _);

        super::future::IntoServiceFuture::new(future)
    }
}
