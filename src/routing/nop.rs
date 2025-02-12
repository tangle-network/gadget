use crate::{JobCall, JobResult};

use bytes::Bytes;
use std::{
    convert::Infallible,
    future::ready,
    task::{Context, Poll},
};
use tower::Service;

/// A [`Service`] that responds with 0 bytes to all requests.
///
/// This is used as the bottom service in a method router. You shouldn't have to
/// use it manually.
#[derive(Clone, Copy, Debug)]
pub(super) struct NoOp;

impl<B> Service<JobCall<B>> for NoOp
where
    B: Send + 'static,
{
    type Response = JobResult;
    type Error = Infallible;
    type Future = std::future::Ready<Result<JobResult, Self::Error>>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: JobCall<B>) -> Self::Future {
        ready(Ok(JobResult::new(Bytes::new())))
    }
}
