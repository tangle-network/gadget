//! Routing between [`Service`]s and handlers.

use crate::future::{Route, RouteFuture};

use crate::IntoMakeService;
use crate::nop::NoOp;
use crate::path_router::JobIdRouter;
use crate::util::try_downcast;
use blueprint_core::{IntoJobId, IntoJobResult, Job, JobCall, JobResult};

use alloc::sync::Arc;
use bytes::Bytes;
use core::fmt;
use core::marker::PhantomData;
use core::task::{Context, Poll};
use tower::{BoxError, Layer, Service};

macro_rules! panic_on_err {
    ($expr:expr) => {
        match $expr {
            Ok(x) => x,
            Err(err) => panic!("{err}"),
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RouteId(pub u32);

/// The router type for composing handlers and services.
#[must_use]
pub struct Router<Ctx = ()> {
    inner: Arc<RouterInner<Ctx>>,
}

impl<Ctx> Clone for Router<Ctx> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

struct RouterInner<Ctx> {
    job_id_router: JobIdRouter<Ctx, false>,
}

impl<Ctx> Default for Router<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Ctx> fmt::Debug for Router<Ctx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router")
            .field("job_id_router", &self.inner.job_id_router)
            .finish()
    }
}

macro_rules! map_inner {
    ( $self_:ident, $inner:pat_param => $expr:expr) => {
        #[allow(redundant_semicolons)]
        {
            let $inner = $self_.into_inner();
            Router {
                inner: Arc::new($expr),
            }
        }
    };
}

macro_rules! tap_inner {
    ( $self_:ident, mut $inner:ident => { $($stmt:stmt)* } ) => {
        #[allow(redundant_semicolons)]
        {
            let mut $inner = $self_.into_inner();
            $($stmt)*
            Router {
                inner: Arc::new($inner),
            }
        }
    };
}

impl<Ctx> Router<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    /// Create a new `Router`.
    ///
    /// Unless you add additional routes this will respond with `404 Not Found` to
    /// all requests.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RouterInner {
                job_id_router: Default::default(),
            }),
        }
    }

    fn into_inner(self) -> RouterInner<Ctx> {
        Arc::try_unwrap(self.inner).unwrap_or_else(|arc| RouterInner {
            job_id_router: arc.job_id_router.clone(),
        })
    }

    #[track_caller]
    pub fn route<I, J, T>(self, job_id: I, job: J) -> Self
    where
        I: IntoJobId,
        J: Job<T, Ctx>,
        T: 'static,
    {
        tap_inner!(self, mut this => {
            panic_on_err!(this.job_id_router.route(job_id, job));
        })
    }

    pub fn route_service<T>(self, job_id: u32, service: T) -> Self
    where
        T: Service<JobCall, Error = BoxError> + Clone + Send + Sync + 'static,
        T::Response: IntoJobResult,
        T::Future: Send + 'static,
    {
        let service = match try_downcast::<Router<Ctx>, _>(service) {
            Ok(_) => {
                panic!(
                    "Invalid route: `Router::route_service` cannot be used with `Router`s. \
                     Use `Router::nest` instead"
                );
            }
            Err(service) => service,
        };

        tap_inner!(self, mut this => {
            panic_on_err!(this.job_id_router.route_service(job_id, service));
        })
    }

    /// Add a [`Job`] with no associated ID
    ///
    /// This is useful for jobs that want to watch for certain events. Any [`JobCall`] with no specified
    /// ID will be passed to every `catch_all` job.
    #[track_caller]
    pub fn catch_all<J, T>(self, job: J) -> Self
    where
        J: Job<T, Ctx>,
        T: 'static,
    {
        tap_inner!(self, mut this => {
            panic_on_err!(this.job_id_router.catch_all(job));
        })
    }

    #[track_caller]
    pub fn merge<R>(self, other: R) -> Self
    where
        R: Into<Router<Ctx>>,
    {
        let other: Router<Ctx> = other.into();
        let RouterInner {
            job_id_router: path_router,
        } = other.into_inner();

        map_inner!(self, mut this => {
            panic_on_err!(this.job_id_router.merge(path_router));
            this
        })
    }

    pub fn layer<L>(self, layer: L) -> Router<Ctx>
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<JobCall> + Clone + Send + Sync + 'static,
        <L::Service as Service<JobCall>>::Response: IntoJobResult + 'static,
        <L::Service as Service<JobCall>>::Error: Into<BoxError> + 'static,
        <L::Service as Service<JobCall>>::Future: Send + 'static,
    {
        map_inner!(self, this => RouterInner {
            job_id_router: this.job_id_router.layer(layer.clone()),
        })
    }

    #[track_caller]
    pub fn route_layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<JobCall> + Clone + Send + Sync + 'static,
        <L::Service as Service<JobCall>>::Response: IntoJobResult + 'static,
        <L::Service as Service<JobCall>>::Error: Into<BoxError> + 'static,
        <L::Service as Service<JobCall>>::Future: Send + 'static,
    {
        map_inner!(self, this => RouterInner {
            job_id_router: this.job_id_router.route_layer(layer),
        })
    }

    /// True if the router currently has at least one route added.
    pub fn has_routes(&self) -> bool {
        self.inner.job_id_router.has_routes()
    }

    pub fn with_context<Ctx2>(self, context: Ctx) -> Router<Ctx2> {
        map_inner!(self, this => RouterInner {
            job_id_router: this.job_id_router.with_context(context.clone()),
        })
    }

    pub(crate) fn call_with_context(&self, call: JobCall, context: Ctx) -> RouteFuture<BoxError> {
        tracing::trace!(?call, "routing a job call to inner routers");
        let (call, context) = match self.inner.job_id_router.call_with_context(call, context) {
            Ok(future) => return future,
            Err((call, context)) => (call, context),
        };

        let (call, _context) = match self.inner.job_id_router.catch_all_call(call, context) {
            Ok(mut futures) => {
                let mut first = futures.remove(0);
                loop {
                    if futures.is_empty() {
                        break;
                    }

                    first.join(futures.remove(0));
                }

                return first;
            }
            Err((call, context)) => (call, context),
        };

        Route::new(NoOp).call(call)
    }

    /// Convert the router into a borrowed [`Service`] with a fixed request body type, to aid type
    /// inference.
    ///
    /// In some cases when calling methods from [`tower::ServiceExt`] on a [`Router`] you might get
    /// type inference errors along the lines of
    ///
    /// ```not_rust
    /// let response = router.ready().await?.call(request).await?;
    ///                       ^^^^^ cannot infer type for type parameter `B`
    /// ```
    ///
    /// This happens because `Router` implements [`Service`] with `impl<B> Service<Request<B>> for Router<()>`.
    ///
    /// For example:
    ///
    /// ```compile_fail
    /// use axum::{
    ///     Router,
    ///     routing::get,
    ///     http::Request,
    ///     body::Body,
    /// };
    /// use tower::{Service, ServiceExt};
    ///
    /// # async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut router = Router::new().route("/", get(|| async {}));
    /// let request = Request::new(Body::empty());
    /// let response = router.ready().await?.call(request).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Calling `Router::as_service` fixes that:
    ///
    /// ```
    /// use blueprint_sdk::{JobCall, Router};
    /// use bytes::Bytes;
    /// use tower::{Service, ServiceExt};
    ///
    /// const MY_JOB_ID: u32 = 0;
    ///
    /// # async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut router = Router::new().route(MY_JOB_ID, || async {});
    /// let request = JobCall::new(MY_JOB_ID, Bytes::new());
    /// let response = router.as_service().ready().await?.call(request).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// This is mainly used when calling `Router` in tests. It shouldn't be necessary when running
    /// the `Router` normally via [`Router::into_make_service`].
    pub fn as_service<B>(&mut self) -> RouterAsService<'_, B, Ctx> {
        RouterAsService {
            router: self,
            _marker: PhantomData,
        }
    }

    /// Convert the router into an owned [`Service`] with a fixed request body type, to aid type
    /// inference.
    ///
    /// This is the same as [`Router::as_service`] instead it returns an owned [`Service`]. See
    /// that method for more details.
    pub fn into_service<B>(self) -> RouterIntoService<B, Ctx> {
        RouterIntoService {
            router: self,
            _marker: PhantomData,
        }
    }
}

impl Router {
    /// Convert this router into a [`MakeService`], that is a [`Service`] whose
    /// response is another service.
    ///
    /// ```
    /// use blueprint_sdk::Router;
    ///
    /// const MY_JOB_ID: u32 = 0;
    ///
    /// let app = Router::new().route(MY_JOB_ID, || async { "Hi!" });
    ///
    /// # async {
    /// let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    /// axum::serve(listener, app).await.unwrap();
    /// # };
    /// ```
    ///
    /// [`MakeService`]: tower::make::MakeService
    pub fn into_make_service(self) -> IntoMakeService<Self> {
        // call `Router::with_state` such that everything is turned into `Route` eagerly
        // rather than doing that per request
        IntoMakeService::new(self.with_context(()))
    }
}

impl<B> Service<JobCall<B>> for Router<()>
where
    B: Into<Bytes>,
{
    type Response = JobResult;
    type Error = BoxError;
    type Future = RouteFuture<BoxError>;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, call: JobCall<B>) -> Self::Future {
        self.call_with_context(call.map(Into::into), ())
    }
}

/// A [`Router`] converted into a borrowed [`Service`] with a fixed body type.
///
/// See [`Router::as_service`] for more details.
pub struct RouterAsService<'a, B, Ctx = ()> {
    router: &'a mut Router<Ctx>,
    _marker: PhantomData<B>,
}

impl<B> Service<JobCall<B>> for RouterAsService<'_, B, ()>
where
    B: Into<Bytes>,
{
    type Response = JobResult;
    type Error = BoxError;
    type Future = RouteFuture<BoxError>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <Router as Service<JobCall<B>>>::poll_ready(self.router, cx)
    }

    #[inline]
    fn call(&mut self, call: JobCall<B>) -> Self::Future {
        self.router.call(call)
    }
}

impl<B, Ctx> fmt::Debug for RouterAsService<'_, B, Ctx>
where
    Ctx: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouterAsService")
            .field("router", &self.router)
            .finish()
    }
}

/// A [`Router`] converted into an owned [`Service`] with a fixed body type.
///
/// See [`Router::into_service`] for more details.
pub struct RouterIntoService<B, Ctx = ()> {
    router: Router<Ctx>,
    _marker: PhantomData<B>,
}

impl<B, Ctx> Clone for RouterIntoService<B, Ctx>
where
    Router<Ctx>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
            _marker: PhantomData,
        }
    }
}

impl<B> Service<JobCall<B>> for RouterIntoService<B, ()>
where
    B: Into<Bytes>,
{
    type Response = JobResult;
    type Error = BoxError;
    type Future = RouteFuture<BoxError>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <Router as Service<JobCall<B>>>::poll_ready(&mut self.router, cx)
    }

    #[inline]
    fn call(&mut self, req: JobCall<B>) -> Self::Future {
        self.router.call(req)
    }
}

impl<B, Ctx> fmt::Debug for RouterIntoService<B, Ctx>
where
    Ctx: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RouterIntoService")
            .field("router", &self.router)
            .finish()
    }
}

#[test]
fn traits() {
    use crate::test_helpers::*;
    assert_send::<Router<()>>();
    assert_sync::<Router<()>>();
}
