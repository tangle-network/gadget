use crate::boxed::BoxedIntoRoute;
use crate::future::{Route, RouteFuture};
use crate::routing::RouteId;
use alloc::borrow::Cow;
use alloc::sync::Arc;
use alloc::vec::Vec;
use blueprint_core::{IntoJobId, IntoJobResult, Job, JobCall, JobId};
use core::fmt;
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

pub(super) struct JobIdRouter<Ctx, const IS_FALLBACK: bool> {
    routes: HashMap<RouteId, Handler<Ctx>>,
    node: Arc<Node>,
    prev_route_id: RouteId,
    catch_all_routes: Vec<Handler<Ctx>>,
}

impl<Ctx, const IS_FALLBACK: bool> JobIdRouter<Ctx, IS_FALLBACK>
where
    Ctx: Clone + Send + Sync + 'static,
{
    pub(super) fn route<I, J, T>(&mut self, job_id: I, job: J) -> Result<(), Cow<'static, str>>
    where
        I: IntoJobId,
        J: Job<T, Ctx>,
        T: 'static,
    {
        let id = self.next_route_id();
        self.set_node(job_id.into_job_id(), id);
        self.routes
            .insert(id, Handler::Boxed(BoxedIntoRoute::from_job(job)));

        Ok(())
    }

    pub(super) fn route_service<I, T>(
        &mut self,
        job_id: I,
        service: T,
    ) -> Result<(), Cow<'static, str>>
    where
        I: IntoJobId,
        T: Service<JobCall, Error = BoxError> + Clone + Send + Sync + 'static,
        T::Response: IntoJobResult,
        T::Future: Send + 'static,
    {
        let id = self.next_route_id();
        self.set_node(job_id.into_job_id(), id);
        self.routes.insert(id, Handler::Route(Route::new(service)));
        Ok(())
    }

    pub(super) fn catch_all<J, T>(&mut self, job: J) -> Result<(), Cow<'static, str>>
    where
        J: Job<T, Ctx>,
        T: 'static,
    {
        self.catch_all_routes
            .push(Handler::Boxed(BoxedIntoRoute::from_job(job)));

        Ok(())
    }

    fn set_node(&mut self, job_id: JobId, id: RouteId) {
        let node = Arc::make_mut(&mut self.node);
        node.insert(job_id, id);
    }

    pub(super) fn merge(
        &mut self,
        other: JobIdRouter<Ctx, IS_FALLBACK>,
    ) -> Result<(), Cow<'static, str>> {
        todo!();
    }

    pub(super) fn layer<L>(self, layer: L) -> JobIdRouter<Ctx, IS_FALLBACK>
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

        let catch_all_routes = self
            .catch_all_routes
            .into_iter()
            .map(|h| h.layer(layer.clone()))
            .collect();

        JobIdRouter {
            routes,
            node: self.node,
            prev_route_id: self.prev_route_id,
            catch_all_routes,
        }
    }

    #[track_caller]
    pub(super) fn route_layer<L>(self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<JobCall> + Clone + Send + Sync + 'static,
        <L::Service as Service<JobCall>>::Response: IntoJobResult + 'static,
        <L::Service as Service<JobCall>>::Error: Into<BoxError> + 'static,
        <L::Service as Service<JobCall>>::Future: Send + 'static,
    {
        todo!()
    }

    pub(super) fn has_routes(&self) -> bool {
        !self.routes.is_empty()
    }

    pub(super) fn with_context<Ctx2>(self, context: Ctx) -> JobIdRouter<Ctx2, IS_FALLBACK> {
        let routes = self
            .routes
            .into_iter()
            .map(|(id, endpoint)| match endpoint {
                Handler::Route(route) => (id, Handler::Route(route)),
                Handler::Boxed(boxed) => (id, Handler::Route(boxed.into_route(context.clone()))),
            })
            .collect();

        let catch_all_routes = self
            .catch_all_routes
            .into_iter()
            .map(|endpoint| match endpoint {
                Handler::Route(route) => Handler::Route(route),
                Handler::Boxed(boxed) => Handler::Route(boxed.into_route(context.clone())),
            })
            .collect();

        JobIdRouter {
            routes,
            node: self.node,
            prev_route_id: self.prev_route_id,
            catch_all_routes,
        }
    }

    pub(super) fn call_with_context(
        &self,
        call: JobCall,
        context: Ctx,
    ) -> Result<RouteFuture<BoxError>, (JobCall, Ctx)> {
        let (parts, body) = call.into_parts();
        let Some(route) = self.node.get(parts.job_id) else {
            return Err((JobCall::from_parts(parts, body), context));
        };

        let handler = self
            .routes
            .get(&route)
            .expect("no route for id. This is a bug in axum. Please file an issue");

        let call = JobCall::from_parts(parts, body);
        match handler {
            Handler::Route(route) => Ok(route.clone().call_owned(call)),
            Handler::Boxed(boxed) => Ok(boxed.clone().into_route(context).call(call)),
        }
    }

    pub(super) fn catch_all_call(
        &self,
        call: JobCall,
        context: Ctx,
    ) -> Result<Vec<RouteFuture<BoxError>>, (JobCall, Ctx)> {
        if self.catch_all_routes.is_empty() {
            return Err((call, context));
        }

        Ok(self
            .catch_all_routes
            .iter()
            .map(|handler| match handler {
                Handler::Route(route) => route.clone().call_owned(call.clone()),
                Handler::Boxed(boxed) => {
                    boxed.clone().into_route(context.clone()).call(call.clone())
                }
            })
            .collect::<Vec<_>>())
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

impl<Ctx, const IS_FALLBACK: bool> Default for JobIdRouter<Ctx, IS_FALLBACK> {
    fn default() -> Self {
        Self {
            routes: Default::default(),
            node: Default::default(),
            prev_route_id: RouteId(0),
            catch_all_routes: Vec::new(),
        }
    }
}

impl<Ctx, const IS_FALLBACK: bool> fmt::Debug for JobIdRouter<Ctx, IS_FALLBACK> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathRouter")
            .field("routes", &self.routes)
            .field("node", &self.node)
            .finish()
    }
}

impl<Ctx, const IS_FALLBACK: bool> Clone for JobIdRouter<Ctx, IS_FALLBACK>
where
    Ctx: Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            routes: self.routes.clone(),
            node: self.node.clone(),
            prev_route_id: self.prev_route_id,
            catch_all_routes: self.catch_all_routes.clone(),
        }
    }
}

#[derive(Clone, Default)]
struct Node {
    route_id_to_job: HashMap<RouteId, JobId>,
    job_to_route_id: HashMap<JobId, RouteId>,
}

impl Node {
    fn insert(&mut self, job_id: JobId, val: RouteId) {
        self.route_id_to_job.insert(val, job_id);
        self.job_to_route_id.insert(job_id, val);
    }

    fn get(&self, job_id: JobId) -> Option<RouteId> {
        self.job_to_route_id.get(&job_id).copied()
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("paths", &self.route_id_to_job)
            .finish()
    }
}
