use crate::JobCall;
use crate::extract::{FromJobCall, FromJobCallParts};
use core::future::Future;

mod sealed {
    use crate::JobCall;

    pub trait Sealed {}
    impl Sealed for JobCall {}
}

/// Extension trait that adds additional methods to [`JobCall`].
pub trait JobCallExt: sealed::Sealed + Sized {
    /// Apply an extractor to this `JobCall`.
    ///
    /// This is just a convenience for `E::from_job_call(call, &())`.
    ///
    /// Note this consumes the job call. Use [`JobCallExt::extract_parts`] if you're not extracting
    /// the body and don't want to consume the job call.
    ///
    /// # Example
    ///
    /// ```
    /// use axum::{
    ///     Form, Json, RequestExt,
    ///     body::Body,
    ///     extract::{FromRequest, Request},
    ///     http::{StatusCode, header::CONTENT_TYPE},
    ///     response::{IntoResponse, Response},
    /// };
    ///
    /// struct FormOrJson<T>(T);
    ///
    /// impl<S, T> FromRequest<S> for FormOrJson<T>
    /// where
    ///     Json<T>: FromRequest<()>,
    ///     Form<T>: FromRequest<()>,
    ///     T: 'static,
    ///     S: Send + Sync,
    /// {
    ///     type Rejection = Response;
    ///
    ///     async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
    ///         let content_type = req
    ///             .headers()
    ///             .get(CONTENT_TYPE)
    ///             .and_then(|value| value.to_str().ok())
    ///             .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?;
    ///
    ///         if content_type.starts_with("application/json") {
    ///             let Json(payload) = req
    ///                 .extract::<Json<T>, _>()
    ///                 .await
    ///                 .map_err(|err| err.into_response())?;
    ///
    ///             Ok(Self(payload))
    ///         } else if content_type.starts_with("application/x-www-form-urlencoded") {
    ///             let Form(payload) = req
    ///                 .extract::<Form<T>, _>()
    ///                 .await
    ///                 .map_err(|err| err.into_response())?;
    ///
    ///             Ok(Self(payload))
    ///         } else {
    ///             Err(StatusCode::BAD_REQUEST.into_response())
    ///         }
    ///     }
    /// }
    /// ```
    fn extract<E, M>(self) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCall<(), M> + 'static,
        M: 'static;

    /// Apply an extractor that requires some context to this `JobCall`.
    ///
    /// This is just a convenience for `E::from_job_call(call, ctx)`.
    ///
    /// Note this consumes the job call. Use [`JobCallExt::extract_parts_with_state`] if you're not
    /// extracting the body and don't want to consume the job call.
    ///
    /// # Example
    ///
    /// ```
    /// use axum::{
    ///     body::Body,
    ///     extract::{Request, FromRef, FromRequest},
    ///     RequestExt,
    /// };
    ///
    /// struct MyExtractor {
    ///     requires_state: RequiresState,
    /// }
    ///
    /// impl<S> FromRequest<S> for MyExtractor
    /// where
    ///     String: FromRef<S>,
    ///     S: Send + Sync,
    /// {
    ///     type Rejection = std::convert::Infallible;
    ///
    ///     async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
    ///         let requires_state = req.extract_with_ctx::<RequiresState, _, _>(state).await?;
    ///
    ///         Ok(Self { requires_state })
    ///     }
    /// }
    ///
    /// // some extractor that consumes the request body and requires state
    /// struct RequiresState { /* ... */ }
    ///
    /// impl<S> FromRequest<S> for RequiresState
    /// where
    ///     String: FromRef<S>,
    ///     S: Send + Sync,
    /// {
    ///     // ...
    ///     # type Rejection = std::convert::Infallible;
    ///     # async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
    ///     #     todo!()
    ///     # }
    /// }
    /// ```
    fn extract_with_ctx<E, Ctx, M>(
        self,
        ctx: &Ctx,
    ) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCall<Ctx, M> + 'static,
        Ctx: Send + Sync;

    /// Apply a parts extractor to this `JobCall`.
    ///
    /// This is just a convenience for `E::from_job_call_parts(parts, ctx)`.
    ///
    /// # Example
    ///
    /// ```
    /// use axum::{
    ///     Json, RequestExt,
    ///     body::Body,
    ///     extract::{FromRequest, Path, Request},
    ///     response::{IntoResponse, Response},
    /// };
    /// use axum_extra::{
    ///     TypedHeader,
    ///     headers::{Authorization, authorization::Bearer},
    /// };
    /// use std::collections::HashMap;
    ///
    /// struct MyExtractor<T> {
    ///     path_params: HashMap<String, String>,
    ///     payload: T,
    /// }
    ///
    /// impl<S, T> FromRequest<S> for MyExtractor<T>
    /// where
    ///     S: Send + Sync,
    ///     Json<T>: FromRequest<()>,
    ///     T: 'static,
    /// {
    ///     type Rejection = Response;
    ///
    ///     async fn from_request(mut req: Request, _state: &S) -> Result<Self, Self::Rejection> {
    ///         let path_params = req
    ///             .extract_parts::<Path<_>>()
    ///             .await
    ///             .map(|Path(path_params)| path_params)
    ///             .map_err(|err| err.into_response())?;
    ///
    ///         let Json(payload) = req
    ///             .extract::<Json<T>, _>()
    ///             .await
    ///             .map_err(|err| err.into_response())?;
    ///
    ///         Ok(Self {
    ///             path_params,
    ///             payload,
    ///         })
    ///     }
    /// }
    /// ```
    fn extract_parts<E>(&mut self) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCallParts<()> + 'static;

    /// Apply a parts extractor that requires some state to this `Request`.
    ///
    /// This is just a convenience for `E::from_job_call_parts(parts, ctx)`.
    ///
    /// # Example
    ///
    /// ```
    /// use axum::{
    ///     extract::{Request, FromRef, FromRequest, FromRequestParts},
    ///     http::request::Parts,
    ///     response::{IntoResponse, Response},
    ///     body::Body,
    ///     Json, RequestExt,
    /// };
    ///
    /// struct MyExtractor<T> {
    ///     requires_state: RequiresState,
    ///     payload: T,
    /// }
    ///
    /// impl<S, T> FromRequest<S> for MyExtractor<T>
    /// where
    ///     String: FromRef<S>,
    ///     Json<T>: FromRequest<()>,
    ///     T: 'static,
    ///     S: Send + Sync,
    /// {
    ///     type Rejection = Response;
    ///
    ///     async fn from_request(mut req: Request, state: &S) -> Result<Self, Self::Rejection> {
    ///         let requires_state = req
    ///             .extract_parts_with_state::<RequiresState, _>(state)
    ///             .await
    ///             .map_err(|err| err.into_response())?;
    ///
    ///         let Json(payload) = req
    ///             .extract::<Json<T>, _>()
    ///             .await
    ///             .map_err(|err| err.into_response())?;
    ///
    ///         Ok(Self {
    ///             requires_state,
    ///             payload,
    ///         })
    ///     }
    /// }
    ///
    /// struct RequiresState {}
    ///
    /// impl<S> FromRequestParts<S> for RequiresState
    /// where
    ///     String: FromRef<S>,
    ///     S: Send + Sync,
    /// {
    ///     // ...
    ///     # type Rejection = std::convert::Infallible;
    ///     # async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
    ///     #     todo!()
    ///     # }
    /// }
    /// ```
    fn extract_parts_with_ctx<'a, E, Ctx>(
        &'a mut self,
        ctx: &'a Ctx,
    ) -> impl Future<Output = Result<E, E::Rejection>> + Send + 'a
    where
        E: FromJobCallParts<Ctx> + 'static,
        Ctx: Send + Sync;
}

impl JobCallExt for JobCall {
    fn extract<E, M>(self) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCall<(), M> + 'static,
        M: 'static,
    {
        self.extract_with_ctx(&())
    }

    fn extract_with_ctx<E, Ctx, M>(
        self,
        ctx: &Ctx,
    ) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCall<Ctx, M> + 'static,
        Ctx: Send + Sync,
    {
        E::from_job_call(self, ctx)
    }

    fn extract_parts<E>(&mut self) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCallParts<()> + 'static,
    {
        self.extract_parts_with_ctx(&())
    }

    async fn extract_parts_with_ctx<'a, E, Ctx>(
        &'a mut self,
        ctx: &'a Ctx,
    ) -> Result<E, E::Rejection>
    where
        E: FromJobCallParts<Ctx> + 'static,
        Ctx: Send + Sync,
    {
        let mut call = crate::job_call::JobCall::default();
        *call.job_id_mut() = self.job_id();
        *call.metadata_mut() = core::mem::take(self.metadata_mut());
        let (mut parts, ()) = call.into_parts();

        let result = E::from_job_call_parts(&mut parts, ctx).await;

        *self.job_id_mut() = parts.job_id;
        *self.metadata_mut() = core::mem::take(&mut parts.metadata);

        result
    }
}
