use super::{Route, RouteId};
use crate::boxed::BoxedIntoRoute;
use crate::routing::future::RouteFuture;
use crate::{IntoJobResult, Job, JobCall};

use alloc::borrow::Cow;
use alloc::sync::Arc;
use core::fmt;
use std::collections::HashMap;
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
}

impl<Ctx> JobIdRouter<Ctx, true>
where
    Ctx: Clone + Send + Sync + 'static,
{
    pub(super) fn new_fallback() -> Self {
        let mut this = Self::default();
        // this.set_fallback(Route::new(NoOp));
        this
    }

    pub(super) fn set_fallback(&mut self, endpoint: Route) {
        todo!("remove, there's no need for a fallback handler. we know all the job IDs (routes)")
    }
}

impl<Ctx, const IS_FALLBACK: bool> JobIdRouter<Ctx, IS_FALLBACK>
where
    Ctx: Clone + Send + Sync + 'static,
{
    pub(super) fn route<J, T>(&mut self, job_id: u32, job: J) -> Result<(), Cow<'static, str>>
    where
        J: Job<T, Ctx>,
        T: 'static,
    {
        let id = self.next_route_id();
        self.set_node(job_id, id)?;
        self.routes
            .insert(id, Handler::Boxed(BoxedIntoRoute::from_job(job)));

        Ok(())
    }

    pub(super) fn route_service<T>(
        &mut self,
        job_id: u32,
        service: T,
    ) -> Result<(), Cow<'static, str>>
    where
        T: Service<JobCall, Error = BoxError> + Clone + Send + Sync + 'static,
        T::Response: IntoJobResult,
        T::Future: Send + 'static,
    {
        let id = self.next_route_id();
        self.set_node(job_id, id)?;
        self.routes.insert(id, Handler::Route(Route::new(service)));
        Ok(())
    }

    fn set_node(&mut self, job_id: u32, id: RouteId) -> Result<(), String> {
        let node = Arc::make_mut(&mut self.node);

        node.insert(job_id, id)
            .map_err(|err| format!("Invalid route {job_id:?}: {err}"))
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

        JobIdRouter {
            routes,
            node: self.node,
            prev_route_id: self.prev_route_id,
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

        JobIdRouter {
            routes,
            node: self.node,
            prev_route_id: self.prev_route_id,
        }
    }

    pub(super) fn call_with_context(
        &self,
        mut call: JobCall,
        context: Ctx,
    ) -> Result<RouteFuture<BoxError>, (JobCall, Ctx)> {
        let (mut parts, body) = call.into_parts();
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

    pub(super) fn replace_route(&mut self, job_id: u32, route: Route) {
        todo!("this can probably be removed, there's no need to replace jobs?");
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
        }
    }
}

/// Wrapper around `matchit::Router` that supports merging two `Router`s.
#[derive(Clone, Default)]
struct Node {
    route_id_to_job: HashMap<RouteId, u32>,
    job_to_route_id: HashMap<u32, RouteId>,
}

impl Node {
    fn insert(&mut self, job_id: u32, val: RouteId) -> Result<(), matchit::InsertError> {
        self.route_id_to_job.insert(val, job_id);
        self.job_to_route_id.insert(job_id, val);

        Ok(())
    }

    fn get(&self, job_id: u32) -> Option<RouteId> {
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

#[track_caller]
fn validate_nest_path(path: &str) -> &str {
    assert!(path.starts_with('/'));
    assert!(path.len() > 1);

    if path.split('/').any(|segment| {
        segment.starts_with("{*") && segment.ends_with('}') && !segment.ends_with("}}")
    }) {
        panic!("Invalid route: nested routes cannot contain wildcards (*)");
    }

    path
}

pub(crate) fn path_for_nested_route<'a>(prefix: &'a str, path: &'a str) -> Cow<'a, str> {
    debug_assert!(prefix.starts_with('/'));
    debug_assert!(path.starts_with('/'));

    if prefix.ends_with('/') {
        format!("{prefix}{}", path.trim_start_matches('/')).into()
    } else if path == "/" {
        prefix.into()
    } else {
        format!("{prefix}{path}").into()
    }
}
