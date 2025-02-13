use crate::{IntoJobResult, JobCall, JobResult};
use bytes::{Buf, Bytes};
use pin_project_lite::pin_project;
use std::{
    convert::Infallible,
    fmt,
    future::Future,
    pin::Pin,
    task::{ready, Context, Poll},
};
use tower::util::{BoxCloneSyncService, MapErrLayer, MapResponseLayer, Oneshot};
use tower::{Layer, Service, ServiceExt};

/// How routes are stored inside a [`Router`](super::Router).
///
/// You normally shouldn't need to care about this type. It's used in
/// [`Router::layer`](super::Router::layer).
pub struct Route<E = Infallible>(BoxCloneSyncService<JobCall, JobResult, E>);

impl<E> Route<E> {
    pub(crate) fn new<T>(svc: T) -> Self
    where
        T: Service<JobCall, Error = E> + Clone + Send + Sync + 'static,
        T::Response: IntoJobResult + 'static,
        T::Future: Send + 'static,
    {
        Self(BoxCloneSyncService::new(
            svc.map_response(IntoJobResult::into_job_result),
        ))
    }

    /// Variant of [`Route::call`] that takes ownership of the route to avoid cloning.
    pub(crate) fn call_owned(self, call: JobCall) -> RouteFuture<E> {
        self.oneshot_inner_owned(call).not_top_level()
    }

    pub(crate) fn oneshot_inner(&mut self, call: JobCall) -> RouteFuture<E> {
        RouteFuture::new(self.0.clone().oneshot(call))
    }

    /// Variant of [`Route::oneshot_inner`] that takes ownership of the route to avoid cloning.
    pub(crate) fn oneshot_inner_owned(self, call: JobCall) -> RouteFuture<E> {
        RouteFuture::new(self.0.oneshot(call))
    }

    pub(crate) fn layer<L, NewError>(self, layer: L) -> Route<NewError>
    where
        L: Layer<Route<E>> + Clone + Send + 'static,
        L::Service: Service<JobCall> + Clone + Send + Sync + 'static,
        <L::Service as Service<JobCall>>::Response: IntoJobResult + 'static,
        <L::Service as Service<JobCall>>::Error: Into<NewError> + 'static,
        <L::Service as Service<JobCall>>::Future: Send + 'static,
        NewError: 'static,
    {
        let layer = (
            MapErrLayer::new(Into::into),
            MapResponseLayer::new(IntoJobResult::into_job_result),
            layer,
        );

        Route::new(layer.layer(self))
    }
}

impl<E> Clone for Route<E> {
    #[track_caller]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<E> fmt::Debug for Route<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Route").finish()
    }
}

impl<B, E> Service<JobCall<B>> for Route<E>
where
    B: Buf + Send + 'static,
{
    type Response = JobResult;
    type Error = E;
    type Future = RouteFuture<E>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, call: JobCall<B>) -> Self::Future {
        self.oneshot_inner(call.map(|b| Bytes::from(b.chunk().to_vec())))
            .not_top_level()
    }
}

pin_project! {
    /// Response future for [`Route`].
    pub struct RouteFuture<E> {
        #[pin]
        inner: Oneshot<BoxCloneSyncService<JobCall, JobResult, E>, JobCall>,
        allow_header: Option<Bytes>,
        top_level: bool,
    }
}

impl<E> RouteFuture<E> {
    fn new(inner: Oneshot<BoxCloneSyncService<JobCall, JobResult, E>, JobCall>) -> Self {
        Self {
            inner,
            allow_header: None,
            top_level: true,
        }
    }

    pub(crate) fn allow_header(mut self, allow_header: Bytes) -> Self {
        self.allow_header = Some(allow_header);
        self
    }

    pub(crate) fn not_top_level(mut self) -> Self {
        self.top_level = false;
        self
    }
}

impl<E> Future for RouteFuture<E> {
    type Output = Result<JobResult, E>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let res = ready!(this.inner.poll(cx))?;

        Poll::Ready(Ok(res))
    }
}

pin_project! {
    /// A [`RouteFuture`] that always yields a [`Response`].
    pub struct InfallibleRouteFuture {
        #[pin]
        future: RouteFuture<Infallible>,
    }
}

impl InfallibleRouteFuture {
    pub(crate) fn new(future: RouteFuture<Infallible>) -> Self {
        Self { future }
    }
}

impl Future for InfallibleRouteFuture {
    type Output = JobResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match futures_util::ready!(self.project().future.poll(cx)) {
            Ok(response) => Poll::Ready(response),
            Err(err) => match err {},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traits() {
        use crate::test_helpers::*;
        assert_send::<Route<()>>();
    }
}
