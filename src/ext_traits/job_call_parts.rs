use crate::extract::FromJobCallParts;
use crate::job_call::Parts;
use core::future::Future;

mod sealed {
    pub trait Sealed {}
    impl Sealed for crate::job_call::Parts {}
}

/// Extension trait that adds additional methods to [`Parts`].
pub trait JobCallPartsExt: sealed::Sealed + Sized {
    /// Apply an extractor to this `Parts`.
    ///
    /// This is just a convenience for `E::from_job_call_parts(parts, &())`.
    ///
    /// # Example
    ///
    /// ```
    /// use blueprint_sdk::job_call::Parts;
    /// use blueprint_sdk::{FromJobCallParts, JobCallPartsExt, JobResult};
    /// use std::collections::HashMap;
    ///
    /// struct MyExtractor {
    ///     path_params: HashMap<String, String>,
    ///     query_params: HashMap<String, String>,
    /// }
    ///
    /// impl<S> FromJobCallParts<S> for MyExtractor
    /// where
    ///     S: Send + Sync,
    /// {
    ///     type Rejection = JobResult;
    ///
    ///     async fn from_job_call_parts(
    ///         parts: &mut Parts,
    ///         state: &S,
    ///     ) -> Result<Self, Self::Rejection> {
    ///         let path_params = parts
    ///             .extract::<Path<HashMap<String, String>>>()
    ///             .await
    ///             .map(|Path(path_params)| path_params)
    ///             .map_err(|err| err.into_response())?;
    ///
    ///         let query_params = parts
    ///             .extract::<Query<HashMap<String, String>>>()
    ///             .await
    ///             .map(|Query(params)| params)
    ///             .map_err(|err| err.into_response())?;
    ///
    ///         Ok(MyExtractor {
    ///             path_params,
    ///             query_params,
    ///         })
    ///     }
    /// }
    /// ```
    fn extract<E>(&mut self) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCallParts<()> + 'static;

    /// Apply an extractor that requires some state to this `Parts`.
    ///
    /// This is just a convenience for `E::from_job_call_parts(parts, state)`.
    ///
    /// # Example
    ///
    /// ```
    /// use blueprint_sdk::extract::FromRef;
    /// use blueprint_sdk::{FromJobCallParts, JobCallPartsExt};
    /// use blueprint_sdk::job_call::Parts;
    ///
    /// struct MyExtractor {
    ///     requires_state: RequiresState,
    /// }
    ///
    /// impl<S> FromJobCallParts<S> for MyExtractor
    /// where
    ///     String: FromRef<S>,
    ///     S: Send + Sync,
    /// {
    ///     type Rejection = std::convert::Infallible;
    ///
    ///     async fn from_job_call_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
    ///         let requires_state = parts
    ///             .extract_with_ctx::<RequiresState, _>(state)
    ///             .await?;
    ///
    ///         Ok(MyExtractor { requires_state })
    ///     }
    /// }
    ///
    /// struct RequiresState { /* ... */ }
    ///
    /// // some extractor that requires a `String` in the state
    /// impl<S> FromJobCallParts<S> for RequiresState
    /// where
    ///     String: FromRef<S>,
    ///     S: Send + Sync,
    /// {
    ///     // ...
    ///     # type Rejection = std::convert::Infallible;
    ///     # async fn from_job_call_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
    ///     #     unimplemented!()
    ///     # }
    /// }
    /// ```
    fn extract_with_ctx<'a, E, Ctx>(
        &'a mut self,
        ctx: &'a Ctx,
    ) -> impl Future<Output = Result<E, E::Rejection>> + Send + 'a
    where
        E: FromJobCallParts<Ctx> + 'static,
        Ctx: Send + Sync;
}

impl JobCallPartsExt for Parts {
    fn extract<E>(&mut self) -> impl Future<Output = Result<E, E::Rejection>> + Send
    where
        E: FromJobCallParts<()> + 'static,
    {
        self.extract_with_ctx(&())
    }

    fn extract_with_ctx<'a, E, Ctx>(
        &'a mut self,
        ctx: &'a Ctx,
    ) -> impl Future<Output = Result<E, E::Rejection>> + Send + 'a
    where
        E: FromJobCallParts<Ctx> + 'static,
        Ctx: Send + Sync,
    {
        E::from_job_call_parts(self, ctx)
    }
}
