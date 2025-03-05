//! Job future types.

use crate::JobCall;
use crate::JobResult;
use crate::error::BoxError;
use core::{future::Future, pin::Pin, task::Context};
use futures_util::future::Map;
use pin_project_lite::pin_project;
use tower::Service;
use tower::util::Oneshot;

type ServiceFutureInner<F> = Map<F, fn(Option<JobResult>) -> Result<Option<JobResult>, BoxError>>;

opaque_future! {
    /// The response future for [`IntoService`](super::IntoService).
    pub type IntoServiceFuture<F> = ServiceFutureInner<F>;
}

type LayeredFutureInner<S> = Map<
    Oneshot<S, JobCall>,
    fn(
        Result<<S as Service<JobCall>>::Response, <S as Service<JobCall>>::Error>,
    ) -> Option<JobResult>,
>;

pin_project! {
    /// The response future for [`Layered`](super::Layered).
    pub struct LayeredFuture<S>
    where
        S: Service<JobCall>,
    {
        #[pin]
        inner: LayeredFutureInner<S>,
    }
}

impl<S> LayeredFuture<S>
where
    S: Service<JobCall>,
{
    pub(super) fn new(inner: LayeredFutureInner<S>) -> Self {
        Self { inner }
    }
}

impl<S> Future for LayeredFuture<S>
where
    S: Service<JobCall>,
{
    type Output = Option<JobResult>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> core::task::Poll<Self::Output> {
        self.project().inner.poll(cx)
    }
}
