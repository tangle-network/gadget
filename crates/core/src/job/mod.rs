//! Async functions that can be used to handle jobs.
//#![doc = include_str!("../docs/handlers_intro.md")]
//!
//! Some examples of handlers:
//!
//! ```rust
//! use blueprint_sdk::Bytes;
//!
//! // Handler that immediately returns an empty `200 OK` response.
//! async fn unit_handler() {}
//!
//! // Handler that immediately returns a `200 OK` response with a plain text
//! // body.
//! async fn string_handler() -> String {
//!     "Hello, World!".to_string()
//! }
//!
//! // Handler that buffers the request body and returns it.
//! //
//! // This works because `Bytes` implements `FromRequest`
//! // and therefore can be used as an extractor.
//! //
//! // `String` and `StatusCode` both implement `IntoResponse` and
//! // therefore `Result<String, StatusCode>` also implements `IntoResponse`
//! async fn echo(body: Bytes) -> Result<String, String> {
//!     if let Ok(string) = String::from_utf8(body.to_vec()) {
//!         Ok(string)
//!     } else {
//!         Err(String::from("Invalid UTF-8"))
//!     }
//! }
//! ```
//!
//! Instead of a direct `String`, it makes sense to use intermediate error type
//! that can ultimately be converted to `Response`. This allows using `?` operator
//! in handlers. See those examples:
//!
//! * [`anyhow-error-response`][anyhow] for generic boxed errors
//! * [`error-handling`][error-handling] for application-specific detailed errors
//!
//! [anyhow]: https://github.com/tokio-rs/axum/blob/main/examples/anyhow-error-response/src/main.rs
//! [error-handling]: https://github.com/tokio-rs/axum/blob/main/examples/error-handling/src/main.rs
//#![doc = include_str!("../docs/debugging_handler_type_errors.md")]

use crate::{
    JobCall, JobResult,
    extract::{FromJobCall, FromJobCallParts},
    job_result::IntoJobResult,
};
use alloc::boxed::Box;
use core::{fmt, future::Future, marker::PhantomData, pin::Pin};
use tower::{Layer, Service, ServiceExt};

pub mod future;
mod service;

pub use self::service::JobService;

/// Trait for async functions that can be used to handle requests.
///
/// You shouldn't need to depend on this trait directly. It is automatically
/// implemented to closures of the right types.
///
/// See the [module docs](crate::handler) for more details.
///
/// # Converting `Handler`s into [`Service`]s
///
/// To convert `Handler`s into [`Service`]s you have to call either
/// [`HandlerWithoutStateExt::into_service`] or [`Handler::with_state`]:
///
/// ```
/// use blueprint_sdk::job::JobWithoutContextExt;
/// use blueprint_sdk::{Context, Job, JobCall};
/// use tower::Service;
///
/// // this handler doesn't require any state
/// async fn one() {}
/// // so it can be converted to a service with `HandlerWithoutStateExt::into_service`
/// assert_service(one.into_service());
///
/// // this handler requires a context
/// async fn two(_: Context<String>) {}
/// // so we have to provide it
/// let handler_with_state = two.with_context(String::new());
/// // which gives us a `Service`
/// assert_service(handler_with_state);
///
/// // helper to check that a value implements `Service`
/// fn assert_service<S>(service: S)
/// where
///     S: Service<JobCall>,
/// {
/// }
/// ```
//#[doc = include_str!("../docs/debugging_handler_type_errors.md")]
///
/// # Handlers that aren't functions
///
/// The `Handler` trait is also implemented for `T: IntoResponse`. That allows easily returning
/// fixed data for routes:
///
/// ```
/// use blueprint_sdk::Router;
/// use serde_json::json;
///
/// const HELLO_JOB_ID: u32 = 0;
/// const USERS_JOB_ID: u32 = 1;
///
/// let app = Router::new()
///     // respond with a fixed string
///     .route(HELLO_JOB_ID, "Hello, World!")
///     // or return some mock data
///     .route(USERS_JOB_ID, json!({ "id": 1, "username": "alice" }).to_string());
/// # let _: Router = app;
/// ```
#[diagnostic::on_unimplemented(
    note = "Consider using `#[blueprint_sdk::debug_handler]` to improve the error message"
)]
pub trait Job<T, Ctx>: Clone + Send + Sync + Sized + 'static {
    /// The type of future calling this handler returns.
    type Future: Future<Output = Option<JobResult>> + Send + 'static;

    /// Call the handler with the given request.
    fn call(self, call: JobCall, ctx: Ctx) -> Self::Future;

    /// Apply a [`tower::Layer`] to the handler.
    ///
    /// All requests to the handler will be processed by the layer's
    /// corresponding middleware.
    ///
    /// This can be used to add additional processing to a request for a single
    /// handler.
    ///
    /// Note this differs from [`routing::Router::layer`](crate::routing::Router::layer)
    /// which adds a middleware to a group of routes.
    ///
    /// If you're applying middleware that produces errors you have to handle the errors
    /// so they're converted into responses. You can learn more about doing that
    /// [here](crate::error_handling).
    ///
    /// # Example
    ///
    /// Adding the [`tower::limit::ConcurrencyLimit`] middleware to a handler
    /// can be done like so:
    ///
    /// ```rust
    /// use blueprint_sdk::{Job, Router};
    /// use tower::limit::{ConcurrencyLimit, ConcurrencyLimitLayer};
    ///
    /// async fn handler() { /* ... */
    /// }
    ///
    /// const MY_JOB_ID: u32 = 0;
    ///
    /// let layered_handler = handler.layer(ConcurrencyLimitLayer::new(64));
    /// let app = Router::new().route(MY_JOB_ID, layered_handler);
    /// # let _: Router = app;
    /// ```
    fn layer<L>(self, layer: L) -> Layered<L, Self, T, Ctx>
    where
        L: Layer<JobService<Self, T, Ctx>> + Clone,
        L::Service: Service<JobCall>,
    {
        Layered {
            layer,
            job: self,
            _marker: PhantomData,
        }
    }

    /// Convert the handler into a [`Service`] by providing the context
    fn with_context(self, ctx: Ctx) -> JobService<Self, T, Ctx> {
        JobService::new(self, ctx)
    }
}

impl<F, Fut, Res, Ctx> Job<((),), Ctx> for F
where
    F: FnOnce() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send,
    Res: IntoJobResult,
{
    type Future = Pin<Box<dyn Future<Output = Option<JobResult>> + Send>>;

    fn call(self, _call: JobCall, _ctx: Ctx) -> Self::Future {
        Box::pin(async move { self().await.into_job_result() })
    }
}

macro_rules! impl_job {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[allow(non_snake_case, unused_mut)]
        impl<F, Fut, Ctx, Res, M, $($ty,)* $last> Job<(M, $($ty,)* $last,), Ctx> for F
        where
            F: FnOnce($($ty,)* $last,) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send,
            Ctx: Send + Sync + 'static,
            Res: IntoJobResult,
            $( $ty: FromJobCallParts<Ctx> + Send, )*
            $last: FromJobCall<Ctx, M> + Send,
        {
            type Future = Pin<Box<dyn Future<Output = Option<JobResult>> + Send>>;

            fn call(self, call: JobCall, state: Ctx) -> Self::Future {
                Box::pin(async move {
                    let (mut parts, body) = call.into_parts();
                    let state = &state;

                    $(
                        let $ty = match $ty::from_job_call_parts(&mut parts, state).await {
                            Ok(value) => value,
                            Err(rejection) => return rejection.into_job_result(),
                        };
                    )*

                    let req = JobCall::from_parts(parts, body);

                    let $last = match $last::from_job_call(req, state).await {
                        Ok(value) => value,
                        Err(rejection) => return rejection.into_job_result(),
                    };

                    let res = self($($ty,)* $last,).await;

                    res.into_job_result()
                })
            }
        }
    };
}

all_the_tuples!(impl_job);

mod private {
    // Marker type for `impl<T: IntoResponse> Job for T`
    #[allow(missing_debug_implementations)]
    pub enum IntoJobResultHandler {}
}

impl<T, Ctx> Job<private::IntoJobResultHandler, Ctx> for T
where
    T: IntoJobResult + Clone + Send + Sync + 'static,
{
    type Future = core::future::Ready<Option<JobResult>>;

    fn call(self, _call: JobCall, _ctx: Ctx) -> Self::Future {
        core::future::ready(self.into_job_result())
    }
}

/// A [`Service`] created from a [`Job`] by applying a Tower middleware.
///
/// Created with [`Job::layer`]. See that method for more details.
pub struct Layered<L, J, T, Ctx> {
    layer: L,
    job: J,
    _marker: PhantomData<fn() -> (T, Ctx)>,
}

impl<L, J, T, Ctx> fmt::Debug for Layered<L, J, T, Ctx>
where
    L: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Layered")
            .field("layer", &self.layer)
            .finish()
    }
}

impl<L, J, T, Ctx> Clone for Layered<L, J, T, Ctx>
where
    L: Clone,
    J: Clone,
{
    fn clone(&self) -> Self {
        Self {
            layer: self.layer.clone(),
            job: self.job.clone(),
            _marker: PhantomData,
        }
    }
}

impl<L, J, Ctx, T> Job<T, Ctx> for Layered<L, J, T, Ctx>
where
    L: Layer<JobService<J, T, Ctx>> + Clone + Send + Sync + 'static,
    L::Service: Service<JobCall> + Clone + Send + 'static,
    <L::Service as Service<JobCall>>::Response: IntoJobResult,
    <L::Service as Service<JobCall>>::Future: Send,
    J: Job<T, Ctx>,
    T: 'static,
    Ctx: 'static,
{
    type Future = future::LayeredFuture<L::Service>;

    fn call(self, req: JobCall, state: Ctx) -> Self::Future {
        use futures_util::future::{FutureExt, Map};

        let svc = self.job.with_context(state);
        let svc = self.layer.layer(svc);

        #[allow(clippy::type_complexity)]
        let future: Map<
            _,
            fn(
                Result<
                    <L::Service as Service<JobCall>>::Response,
                    <L::Service as Service<JobCall>>::Error,
                >,
            ) -> _,
        > = svc.oneshot(req).map(|result| match result {
            Ok(res) => res.into_job_result(),
            Err(_err) => todo!("JobService needs to return a result"),
        });

        future::LayeredFuture::new(future)
    }
}

/// Extension trait for [`Job`]s that don't have state.
///
/// This provides convenience methods to convert the [`Job`] into a [`Service`] or [`MakeService`].
///
/// [`MakeService`]: tower::make::MakeService
pub trait JobWithoutContextExt<T>: Job<T, ()> {
    /// Convert the handler into a [`Service`] and no state.
    fn into_service(self) -> JobService<Self, T, ()>;
}

impl<H, T> JobWithoutContextExt<T> for H
where
    H: Job<T, ()>,
{
    fn into_service(self) -> JobService<Self, T, ()> {
        self.with_context(())
    }
}
