//! Routing between [`Service`]s and jobs.

use crate::future::{Route, RouteFuture};
use alloc::boxed::Box;

use crate::job_id_router::JobIdRouter;
use crate::util::try_downcast;
use blueprint_core::{IntoJobId, IntoJobResult, Job, JobCall, JobResult};

use alloc::sync::Arc;
use alloc::vec::Vec;
use bytes::Bytes;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::{fmt, iter};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
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

/// The router type for composing jobs and services.
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
    job_id_router: JobIdRouter<Ctx>,
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
    ($self_:ident, $inner:pat_param => $expr:expr) => {
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
    /// Unless you add additional routes this will ignore all requests.
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

    /// Add a [`Job`] to the router, with the given job ID.
    ///
    /// The job will be called when a [`JobCall`] with the given job ID is received by the router.
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
                panic!("Invalid route: `Router::route_service` cannot be used with `Router`s.");
            }
            Err(service) => service,
        };

        tap_inner!(self, mut this => {
            panic_on_err!(this.job_id_router.route_service(job_id, service));
        })
    }

    /// Add a [`Job`] that *always* gets called, regardless of the job ID
    ///
    /// This is useful for jobs that want to watch for certain events. Any [`JobCall`] received by
    /// router will be passed to the `job`, regardless if another route matches.
    #[track_caller]
    pub fn always<J, T>(self, job: J) -> Self
    where
        J: Job<T, Ctx>,
        T: 'static,
    {
        tap_inner!(self, mut this => {
            panic_on_err!(this.job_id_router.always(job));
        })
    }

    /// Add a [`Job`] that gets called if no other route matches
    ///
    /// NOTE: This will replace any existing fallback route.
    ///
    /// This will **only** be called when:
    /// - No other route matches the job ID
    /// - No [`always`] route is present
    ///
    /// [`always`]: Router::always
    #[track_caller]
    pub fn fallback<J, T>(self, job: J) -> Self
    where
        J: Job<T, Ctx>,
        T: 'static,
    {
        tap_inner!(self, mut this => {
            panic_on_err!(this.job_id_router.fallback(job));
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

    /// True if the router currently has at least one route added.
    pub fn has_routes(&self) -> bool {
        self.inner.job_id_router.has_routes()
    }

    pub fn with_context<Ctx2>(self, context: Ctx) -> Router<Ctx2> {
        map_inner!(self, this => RouterInner {
            job_id_router: this.job_id_router.with_context(context.clone()),
        })
    }

    pub(crate) fn call_with_context(
        &self,
        call: JobCall,
        context: Ctx,
    ) -> Option<FuturesUnordered<RouteFuture<BoxError>>> {
        blueprint_core::trace!(
            job_id = %call.job_id(),
            metadata = ?call.metadata(),
            body = ?call.body(),
            "routing a job call to inner routers"
        );
        let (call, context) = match self.inner.job_id_router.call_with_context(call, context) {
            Ok(matched_call_future) => {
                blueprint_core::trace!(
                    matched_calls = matched_call_future.len(),
                    "A route matched this job call"
                );
                return Some(matched_call_future);
            }
            Err((call, context)) => (call, context),
        };

        // At this point, no route matched the job ID, and there are no always routes
        blueprint_core::trace!(
            ?call,
            "No explicit or always route caught this job call, passing to fallback"
        );

        self.inner
            .job_id_router
            .call_fallback(call, context)
            .map(|future| FuturesUnordered::from_iter(iter::once(future)))
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
    /// use blueprint_sdk::{Router, JobCall, Bytes};
    /// use tower::{Service, ServiceExt};
    ///
    /// const MY_JOB_ID: u8 = 0;
    ///
    /// # async fn async_main() -> Result<(), blueprint_sdk::error::BoxError> {
    /// let mut router = Router::new().route(MY_JOB_ID, || async {});
    /// let request = JobCall::new(MY_JOB_ID, Bytes::new());
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
    /// # async fn async_main() -> Result<(), blueprint_sdk::error::BoxError> {
    /// let mut router = Router::new().route(MY_JOB_ID, || async {});
    /// let request = JobCall::new(MY_JOB_ID, Bytes::new());
    /// let response = router.as_service().ready().await?.call(request).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// This is mainly used when calling `Router` in tests. It shouldn't be necessary when running
    /// the `Router` normally via the blueprint runner.
    pub fn as_service<B>(&mut self) -> RouterAsService<'_, B, Ctx> {
        RouterAsService {
            router: self,
            _marker: PhantomData,
        }
    }
}

impl<B> Service<JobCall<B>> for Router<()>
where
    B: Into<Bytes>,
{
    type Response = Option<Vec<JobResult>>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, call: JobCall<B>) -> Self::Future {
        let Some(mut futures) = self.call_with_context(call.map(Into::into), ()) else {
            return Box::pin(async { Ok(None) });
        };

        Box::pin(async move {
            let mut results = Vec::with_capacity(futures.len());
            while let Some(item) = futures.next().await {
                blueprint_core::trace!(outcome = ?item, "Job finished with outcome");
                match item {
                    Ok(Some(job)) => results.push(job),
                    // Job produced nothing, and didn't error. Don't include it.
                    Ok(None) => continue,
                    Err(e) => {
                        blueprint_core::error!(?e, "Job failed");
                        return Err(e);
                    }
                }
            }

            Ok(Some(results))
        })
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
    type Response = Option<Vec<JobResult>>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

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

#[test]
fn traits() {
    use crate::test_helpers::*;
    assert_send::<Router<()>>();
    assert_sync::<Router<()>>();
}
