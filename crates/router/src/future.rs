use blueprint_core::{IntoJobResult, JobCall, JobResult};
use bytes::Bytes;
use core::{
    fmt,
    future::Future,
    pin::Pin,
    task::{Context, Poll, ready},
};
use pin_project_lite::pin_project;
use tower::util::{BoxCloneSyncService, MapErrLayer, MapResponseLayer, Oneshot};
use tower::{BoxError, Layer, Service, ServiceExt};

/// How routes are stored inside a [`Router`](super::Router).
///
/// You normally shouldn't need to care about this type. It's used in
/// [`Router::layer`](super::Router::layer).
pub struct Route<E = BoxError>(BoxCloneSyncService<JobCall, Option<JobResult>, E>);

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
        self.oneshot_inner_owned(call)
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
    B: Into<Bytes>,
{
    type Response = Option<JobResult>;
    type Error = E;
    type Future = RouteFuture<E>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, call: JobCall<B>) -> Self::Future {
        self.oneshot_inner(call.map(Into::into))
    }
}

pin_project! {
    /// Response future for [`Route`].
    pub struct RouteFuture<E> {
        #[pin]
        inner: Oneshot<BoxCloneSyncService<JobCall, Option<JobResult>, E>, JobCall>,
    }
}

impl<E> RouteFuture<E> {
    fn new(inner: Oneshot<BoxCloneSyncService<JobCall, Option<JobResult>, E>, JobCall>) -> Self {
        Self { inner }
    }
}

impl<E> Future for RouteFuture<E> {
    type Output = Result<Option<JobResult>, E>;

    #[inline]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let res = ready!(this.inner.poll(cx))?;

        Poll::Ready(Ok(res))
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
