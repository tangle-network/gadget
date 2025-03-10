use crate::boxed::BoxedIntoRoute;
use crate::future::{Route, RouteFuture};
use crate::routing::RouteId;
use alloc::vec::Vec;
use blueprint_core::{IntoJobId, IntoJobResult, Job, JobCall, JobId};
use core::{fmt, iter};
use futures::stream::FuturesUnordered;
use hashbrown::HashMap;
use tower::{BoxError, Layer, Service};

enum Handler<Ctx> {
    Route(Route),
    Boxed(BoxedIntoRoute<Ctx, BoxError>),
}

impl<Ctx> Handler<Ctx> {
    fn layer<L>(self, layer: L) -> Handler<Ctx>
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<JobCall> + Clone + Send + Sync + 'static,
        <L::Service as Service<JobCall>>::Response: IntoJobResult + 'static,
        <L::Service as Service<JobCall>>::Error: Into<BoxError> + 'static,
        <L::Service as Service<JobCall>>::Future: Send + 'static,
        Ctx: 'static,
    {
        match self {
            Handler::Route(route) => Handler::Route(route.layer(layer)),
            Handler::Boxed(boxed) => Handler::Boxed(boxed.layer(layer)),
        }
    }
}

impl<Ctx> fmt::Debug for Handler<Ctx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Handler::Route(route) => route.fmt(f),
            Handler::Boxed(boxed) => boxed.fmt(f),
        }
    }
}

impl<Ctx> Clone for Handler<Ctx> {
    fn clone(&self) -> Self {
        match self {
            Handler::Route(route) => Handler::Route(route.clone()),
            Handler::Boxed(boxed) => Handler::Boxed(boxed.clone()),
        }
    }
}

pub(super) struct JobIdRouter<Ctx> {
    routes: HashMap<RouteId, Handler<Ctx>>,
    job_to_route_id: HashMap<JobId, RouteId>,

    prev_route_id: RouteId,
    // Routes that are *always* called, regardless of job ID
    always_routes: Vec<Handler<Ctx>>,
    // Only used if `IS_FALLBACK` is true
    fallback: Option<Handler<Ctx>>,
}

impl<Ctx> JobIdRouter<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    pub(super) fn route<I, J, T>(&mut self, job_id: I, job: J)
    where
        I: IntoJobId,
        J: Job<T, Ctx>,
        T: 'static,
    {
        let id = self.next_route_id();
        self.job_to_route_id.insert(job_id.into_job_id(), id);
        self.routes
            .insert(id, Handler::Boxed(BoxedIntoRoute::from_job(job)));
    }

    pub(super) fn route_service<I, T>(&mut self, job_id: I, service: T)
    where
        I: IntoJobId,
        T: Service<JobCall, Error = BoxError> + Clone + Send + Sync + 'static,
        T::Response: IntoJobResult,
        T::Future: Send + 'static,
    {
        let id = self.next_route_id();
        self.job_to_route_id.insert(job_id.into_job_id(), id);
        self.routes.insert(id, Handler::Route(Route::new(service)));
    }

    pub(super) fn always<J, T>(&mut self, job: J)
    where
        J: Job<T, Ctx>,
        T: 'static,
    {
        let _ = self.fallback.take();
        self.always_routes
            .push(Handler::Boxed(BoxedIntoRoute::from_job(job)));
    }

    pub(super) fn layer<L>(self, layer: L) -> JobIdRouter<Ctx>
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<JobCall> + Clone + Send + Sync + 'static,
        <L::Service as Service<JobCall>>::Response: IntoJobResult + 'static,
        <L::Service as Service<JobCall>>::Error: Into<BoxError> + 'static,
        <L::Service as Service<JobCall>>::Future: Send + 'static,
    {
        let routes = self
            .routes
            .into_iter()
            .map(|(id, h)| {
                let route = h.layer(layer.clone());
                (id, route)
            })
            .collect();

        let always_routes = self
            .always_routes
            .into_iter()
            .map(|h| h.layer(layer.clone()))
            .collect();

        let fallback = self.fallback.map(|h| h.layer(layer.clone()));

        JobIdRouter {
            routes,
            job_to_route_id: self.job_to_route_id,
            prev_route_id: self.prev_route_id,
            always_routes,
            fallback,
        }
    }

    pub(super) fn has_routes(&self) -> bool {
        !self.routes.is_empty()
    }

    pub(super) fn with_context<Ctx2>(self, context: Ctx) -> JobIdRouter<Ctx2> {
        let routes = self
            .routes
            .into_iter()
            .map(|(id, endpoint)| match endpoint {
                Handler::Route(route) => (id, Handler::Route(route)),
                Handler::Boxed(boxed) => (id, Handler::Route(boxed.into_route(context.clone()))),
            })
            .collect();

        let always_routes = self
            .always_routes
            .into_iter()
            .map(|endpoint| match endpoint {
                Handler::Route(route) => Handler::Route(route),
                Handler::Boxed(boxed) => Handler::Route(boxed.into_route(context.clone())),
            })
            .collect();

        let fallback = self.fallback.map(|fallback| match fallback {
            Handler::Route(route) => Handler::Route(route),
            Handler::Boxed(boxed) => Handler::Route(boxed.into_route(context.clone())),
        });

        JobIdRouter {
            routes,
            job_to_route_id: self.job_to_route_id,
            prev_route_id: self.prev_route_id,
            always_routes,
            fallback,
        }
    }

    pub(super) fn call_with_context(
        &self,
        call: JobCall,
        context: Ctx,
    ) -> Result<FuturesUnordered<RouteFuture<BoxError>>, (JobCall, Ctx)> {
        let (parts, body) = call.into_parts();
        let Some(route) = self.job_to_route_id.get(&parts.job_id) else {
            let call = JobCall::from_parts(parts, body);

            blueprint_core::trace!(?call, "No job found, checking always routes");
            let catch_all_futures = self.call_always_routes(call, context)?;
            return Ok(catch_all_futures.collect::<FuturesUnordered<_>>());
        };

        blueprint_core::trace!("Job found for id {:?}", parts.job_id);

        let handler = self
            .routes
            .get(route)
            .expect("no route for id. This is a bug in blueprint-sdk. Please file an issue");

        let call = JobCall::from_parts(parts, body);
        let matched_call_future = match handler {
            Handler::Route(route) => route.clone().call_owned(call.clone()),
            Handler::Boxed(boxed) => boxed.clone().into_route(context.clone()).call(call.clone()),
        };

        match self.call_always_routes(call, context) {
            Ok(catch_all_futures) => Ok(iter::once(matched_call_future)
                .chain(catch_all_futures)
                .collect::<FuturesUnordered<_>>()),
            Err(_) => Ok(iter::once(matched_call_future).collect::<FuturesUnordered<_>>()),
        }
    }

    fn call_always_routes(
        &self,
        call: JobCall,
        context: Ctx,
    ) -> Result<impl Iterator<Item = RouteFuture<BoxError>> + '_, (JobCall, Ctx)> {
        if self.always_routes.is_empty() {
            return Err((call, context));
        }

        Ok(self.always_routes.iter().map(move |handler| match handler {
            Handler::Route(route) => route.clone().call_owned(call.clone()),
            Handler::Boxed(boxed) => boxed.clone().into_route(context.clone()).call(call.clone()),
        }))
    }

    fn next_route_id(&mut self) -> RouteId {
        let next_id = self
            .prev_route_id
            .0
            .checked_add(1)
            .expect("Over `u32::MAX` routes created. If you need this, please file an issue.");
        self.prev_route_id = RouteId(next_id);
        self.prev_route_id
    }
}

impl<Ctx> JobIdRouter<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    pub(super) fn fallback<J, T>(&mut self, job: J)
    where
        J: Job<T, Ctx>,
        T: 'static,
    {
        self.fallback = Some(Handler::Boxed(BoxedIntoRoute::from_job(job)));
    }

    pub(super) fn call_fallback(
        &self,
        call: JobCall,
        context: Ctx,
    ) -> Option<RouteFuture<BoxError>> {
        let fallback = self.fallback.clone()?;

        match fallback {
            Handler::Route(route) => Some(route.clone().call_owned(call)),
            Handler::Boxed(boxed) => Some(boxed.clone().into_route(context).call(call)),
        }
    }
}

impl<Ctx> Default for JobIdRouter<Ctx> {
    fn default() -> Self {
        Self {
            routes: HashMap::default(),
            job_to_route_id: HashMap::default(),
            prev_route_id: RouteId(0),
            always_routes: Vec::new(),
            fallback: None,
        }
    }
}

impl<Ctx> fmt::Debug for JobIdRouter<Ctx> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JobIdRouter")
            .field("routes", &self.routes)
            .field("always_routes", &self.always_routes)
            .field("fallback", &self.fallback)
            .finish_non_exhaustive()
    }
}

impl<Ctx> Clone for JobIdRouter<Ctx>
where
    Ctx: Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            routes: self.routes.clone(),
            job_to_route_id: self.job_to_route_id.clone(),
            prev_route_id: self.prev_route_id,
            always_routes: self.always_routes.clone(),
            fallback: self.fallback.clone(),
        }
    }
}
