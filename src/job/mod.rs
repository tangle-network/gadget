//! Async functions that can be used to handle requests.
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
    extract::{FromJobCall, FromJobCallParts},
    job_result::IntoJobResult,
    JobCall, JobResult,
};
use alloc::boxed::Box;
use core::{convert::Infallible, fmt, future::Future, marker::PhantomData, pin::Pin};
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
/// use blueprint_sdk::Context;
/// use tower_service::Service;
///
/// // this handler doesn't require any state
/// async fn one() {}
/// // so it can be converted to a service with `HandlerWithoutStateExt::into_service`
/// assert_service(one.into_service());
///
/// // this handler requires a context
/// async fn two(_: Context<String>) {}
/// // so we have to provide it
/// let handler_with_state = two.with_state(String::new());
/// // which gives us a `Service`
/// assert_service(handler_with_state);
///
/// // helper to check that a value implements `Service`
/// fn assert_service<S>(service: S)
/// where
///     S: Service<Request>,
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
/// use blueprint_sdk::{
///     Router,
///     routing::{get, post},
///     Json,
///     http::StatusCode,
/// };
/// use serde_json::json;
///
/// let app = Router::new()
///     // respond with a fixed string
///     .route("/", get("Hello, World!"))
///     // or return some mock data
///     .route("/users", post((
///         StatusCode::CREATED,
///         Json(json!({ "id": 1, "username": "alice" })),
///     )));
/// # let _: Router = app;
/// ```
#[diagnostic::on_unimplemented(
    note = "Consider using `#[axum::debug_handler]` to improve the error message"
)]
pub trait Job<T, Ctx>: Clone + Send + Sync + Sized + 'static {
    /// The type of future calling this handler returns.
    type Future: Future<Output = JobResult> + Send + 'static;

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
    /// use blueprint_sdk::Job;
    /// use tower::limit::{ConcurrencyLimit, ConcurrencyLimitLayer};
    ///
    /// async fn handler() { /* ... */
    /// }
    ///
    /// let layered_handler = handler.layer(ConcurrencyLimitLayer::new(64));
    /// let app = Router::new().route("/", get(layered_handler));
    /// # let _: Router = app;
    /// ```
    fn layer<L>(self, layer: L) -> Layered<L, Self, T, Ctx>
    where
        L: Layer<JobService<Self, T, Ctx>> + Clone,
        L::Service: Service<JobCall>,
    {
        Layered {
            layer,
            handler: self,
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
    type Future = Pin<Box<dyn Future<Output = JobResult> + Send>>;

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
            type Future = Pin<Box<dyn Future<Output = JobResult> + Send>>;

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
    type Future = core::future::Ready<JobResult>;

    fn call(self, _call: JobCall, _ctx: Ctx) -> Self::Future {
        core::future::ready(self.into_job_result())
    }
}

/// A [`Service`] created from a [`Job`] by applying a Tower middleware.
///
/// Created with [`Job::layer`]. See that method for more details.
pub struct Layered<L, H, T, S> {
    layer: L,
    handler: H,
    _marker: PhantomData<fn() -> (T, S)>,
}

impl<L, H, T, Ctx> fmt::Debug for Layered<L, H, T, Ctx>
where
    L: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Layered")
            .field("layer", &self.layer)
            .finish()
    }
}

impl<L, H, T, Ctx> Clone for Layered<L, H, T, Ctx>
where
    L: Clone,
    H: Clone,
{
    fn clone(&self) -> Self {
        Self {
            layer: self.layer.clone(),
            handler: self.handler.clone(),
            _marker: PhantomData,
        }
    }
}

impl<H, Ctx, T, L> Job<T, Ctx> for Layered<L, H, T, Ctx>
where
    L: Layer<JobService<H, T, Ctx>> + Clone + Send + Sync + 'static,
    H: Job<T, Ctx>,
    L::Service: Service<JobCall, Error = Infallible> + Clone + Send + 'static,
    <L::Service as Service<JobCall>>::Response: IntoJobResult,
    <L::Service as Service<JobCall>>::Future: Send,
    T: 'static,
    Ctx: 'static,
{
    type Future = future::LayeredFuture<L::Service>;

    fn call(self, req: JobCall, state: Ctx) -> Self::Future {
        use futures_util::future::{FutureExt, Map};

        let svc = self.handler.with_context(state);
        let svc = self.layer.layer(svc);

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
            Err(err) => match err {},
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
