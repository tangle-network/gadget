use crate::extract::{FromJobCallParts, FromRef};
use crate::job_call::Parts;
use core::{
    convert::Infallible,
    ops::{Deref, DerefMut},
};

/// Extractor for context.
///
/// See ["Accessing context in middleware"][context-from-middleware] for how to
/// access context in middleware.
///
/// context is global and used in every request a router with context receives.
/// For accessing data derived from requests, such as authorization data, see [`Extension`].
///
/// [context-from-middleware]: crate::middleware#accessing-context-in-middleware
/// [`Extension`]: crate::Extension
///
/// # With `Router`
///
/// ```
/// use blueprint_sdk::extract::Context;
/// use blueprint_sdk::{Job, Router};
///
/// // the application context
/// //
/// // here you can put configuration, database connection pools, or whatever
/// // context you need
/// #[derive(Clone)]
/// struct AppContext {}
///
/// let context = AppContext {};
///
/// const MY_JOB_ID: u32 = 0;
///
/// // create a `Router` that holds our context
/// let app = Router::new()
///     .route(MY_JOB_ID, handler)
///     // provide the context so the router can access it
///     .with_context(context);
///
/// async fn handler(
///     // access the context via the `Context` extractor
///     // extracting a context of the wrong type results in a compile error
///     Context(context): Context<AppContext>,
/// ) {
///     // use `context`...
/// }
/// # let _: Router = app;
/// ```
///
/// Note that `context` is an extractor, so be sure to put it before any body
/// extractors, see ["the order of extractors"][order-of-extractors].
///
/// [order-of-extractors]: crate::extract#the-order-of-extractors
///
/// # With `Job`
///
/// ```
/// use blueprint_sdk::Job;
/// use blueprint_sdk::extract::Context;
///
/// #[derive(Clone)]
/// struct AppContext {}
///
/// let context = AppContext {};
///
/// async fn job(Context(context): Context<AppContext>) {
///     // use `context`...
/// }
///
/// // provide the context so the job can access it
/// let job_with_context = job.with_context(context);
/// ```
///
/// # Substates
///
/// [`context`] only allows a single context type but you can use [`FromRef`] to extract "substates":
///
/// ```
/// use blueprint_sdk::Router;
/// use blueprint_sdk::extract::{Context, FromRef};
///
/// // the application context
/// #[derive(Clone)]
/// struct AppContext {
///     // that holds some api specific context
///     api_state: ApiContext,
/// }
///
/// // the api specific context
/// #[derive(Clone)]
/// struct ApiContext {}
///
/// // support converting an `AppContext` in an `ApiContext`
/// impl FromRef<AppContext> for ApiContext {
///     fn from_ref(app_state: &AppContext) -> ApiContext {
///         app_state.api_state.clone()
///     }
/// }
///
/// let context = AppContext {
///     api_state: ApiContext {},
/// };
///
/// const HANDLER_JOB_ID: u32 = 0;
/// const FETCH_API_JOB_ID: u32 = 1;
///
/// let app = Router::new()
///     .route(HANDLER_JOB_ID, handler)
///     .route(FETCH_API_JOB_ID, fetch_api)
///     .with_context(context);
///
/// async fn fetch_api(
///     // access the api specific context
///     Context(api_state): Context<ApiContext>,
/// ) {
/// }
///
/// async fn handler(
///     // we can still access to top level context
///     Context(context): Context<AppContext>,
/// ) {
/// }
/// # let _: Router = app;
/// ```
///
/// For convenience `FromRef` can also be derived using `#[derive(FromRef)]`.
///
/// # For library authors
///
/// If you're writing a library that has an extractor that needs context, this is the recommended way
/// to do it:
///
/// ```rust
/// use blueprint_sdk::extract::{FromJobCallParts, FromRef};
/// use blueprint_sdk::job_call::Parts;
/// use std::convert::Infallible;
///
/// // the extractor your library provides
/// struct MyLibraryExtractor;
///
/// impl<S> FromJobCallParts<S> for MyLibraryExtractor
/// where
///     // keep `S` generic but require that it can produce a `MyLibraryContext`
///     // this means users will have to implement `FromRef<UserContext> for MyLibraryContext`
///     MyLibraryContext: FromRef<S>,
///     S: Send + Sync,
/// {
///     type Rejection = Infallible;
///
///     async fn from_job_call_parts(
///         parts: &mut Parts,
///         context: &S,
///     ) -> Result<Self, Self::Rejection> {
///         // get a `MyLibraryContext` from a reference to the context
///         let context = MyLibraryContext::from_ref(context);
///
///         // ...
///         # todo!()
///     }
/// }
///
/// // the context your library needs
/// struct MyLibraryContext {
///     // ...
/// }
/// ```
///
/// # Shared mutable context
///
/// [As context is global within a `Router`][global] you can't directly get a mutable reference to
/// the context.
///
/// The most basic solution is to use an `Arc<Mutex<_>>`. Which kind of mutex you need depends on
/// your use case. See [the tokio docs] for more details.
///
/// Note that holding a locked `std::sync::Mutex` across `.await` points will result in `!Send`
/// futures which are incompatible with `blueprint_sdk`. If you need to hold a mutex across `.await` points,
/// consider using a `tokio::sync::Mutex` instead.
///
/// ## Example
///
/// ```
/// use blueprint_sdk::Router;
/// use blueprint_sdk::extract::Context;
/// use std::sync::{Arc, Mutex};
///
/// #[derive(Clone)]
/// struct AppContext {
///     data: Arc<Mutex<String>>,
/// }
///
/// async fn job(Context(context): Context<AppContext>) {
///     {
///         let mut data = context.data.lock().expect("mutex was poisoned");
///         *data = "updated foo".to_owned();
///     }
///
///     // ...
/// }
///
/// let context = AppContext {
///     data: Arc::new(Mutex::new("foo".to_owned())),
/// };
///
/// let app = Router::new().route("/", job).with_context(context);
/// # let _: Router = app;
/// ```
///
/// [global]: crate::Router::with_state
/// [the tokio docs]: https://docs.rs/tokio/1.25.0/tokio/sync/struct.Mutex.html#which-kind-of-mutex-should-you-use
#[derive(Debug, Default, Clone, Copy)]
pub struct Context<S>(pub S);

impl<OuterContext, InnerContext> FromJobCallParts<OuterContext> for Context<InnerContext>
where
    InnerContext: FromRef<OuterContext>,
    OuterContext: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_job_call_parts(
        _parts: &mut Parts,
        ctx: &OuterContext,
    ) -> Result<Self, Self::Rejection> {
        let inner_state = InnerContext::from_ref(ctx);
        Ok(Self(inner_state))
    }
}

impl<S> Deref for Context<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> DerefMut for Context<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
