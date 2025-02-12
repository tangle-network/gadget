use super::nop::NoOp;
use super::{Route, RouteId};
use super::{FALLBACK_PARAM_PATH, NEST_TAIL_PARAM};
use crate::routing::future::RouteFuture;
use crate::routing::strip_prefix::StripPrefix;
use crate::{IntoJobResult, Job, JobCall};

use alloc::borrow::Cow;
use core::fmt;
use matchit::MatchError;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tower::{Layer, Service};

pub(super) struct JobIdRouter<const IS_FALLBACK: bool> {
    routes: HashMap<RouteId, Route>,
    node: Arc<Node>,
    prev_route_id: RouteId,
}

impl JobIdRouter<true> {
    pub(super) fn new_fallback() -> Self {
        let mut this = Self::default();
        // this.set_fallback(Route::new(NoOp));
        this
    }

    pub(super) fn set_fallback(&mut self, endpoint: Route) {
        todo!("remove, there's no need for a fallback handler. we know all the job IDs (routes)")
    }
}

impl<const IS_FALLBACK: bool> JobIdRouter<IS_FALLBACK> {
    pub(super) fn route<J, T, Ctx>(&mut self, job_id: u32, job: J) -> Result<(), Cow<'static, str>>
    where
        J: Job<T, Ctx>,
        Ctx: Clone + Send + Sync + 'static,
        T: 'static,
    {
        let id = self.next_route_id();
        self.set_node(job_id, id)?;
        self.routes.insert(id, Route::new(job));

        Ok(())
    }

    pub(super) fn route_service<T>(
        &mut self,
        job_id: u32,
        service: T,
    ) -> Result<(), Cow<'static, str>>
    where
        T: Service<JobCall, Error = Infallible> + Clone + Send + Sync + 'static,
        T::Response: IntoJobResult,
        T::Future: Send + 'static,
    {
        let id = self.next_route_id();
        self.set_node(job_id, id)?;
        self.routes.insert(id, Route::new(service));
        Ok(())
    }

    fn set_node(&mut self, job_id: u32, id: RouteId) -> Result<(), String> {
        let node = Arc::make_mut(&mut self.node);

        node.insert(job_id, id)
            .map_err(|err| format!("Invalid route {job_id:?}: {err}"))
    }

    pub(super) fn merge(
        &mut self,
        other: JobIdRouter<IS_FALLBACK>,
    ) -> Result<(), Cow<'static, str>> {
        let JobIdRouter {
            routes,
            node,
            prev_route_id: _,
        } = other;

        for (id, route) in routes {
            let job_id = node
                .route_id_to_job
                .get(&id)
                .copied()
                .expect("no path for route id. This is a bug in axum. Please file an issue");

            if IS_FALLBACK {
                // when merging two routers it doesn't matter if you do `a.merge(b)` or
                // `b.merge(a)`. This must also be true for fallbacks.
                //
                // However all fallback routers will have routes for `/` and `/*` so when merging
                // we have to ignore the top level fallbacks on one side otherwise we get
                // conflicts.
                //
                // `Router::merge` makes sure that when merging fallbacks `other` always has the
                // fallback we want to keep. It panics if both routers have a custom fallback. Thus
                // it is always okay to ignore one fallback and `Router::merge` also makes sure the
                // one we can ignore is that of `self`.
                self.replace_route(job_id, route);
            } else {
                self.route_service(job_id, route)?
            }
        }

        Ok(())
    }

    pub(super) fn layer<L>(self, layer: L) -> JobIdRouter<IS_FALLBACK>
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<JobCall> + Clone + Send + Sync + 'static,
        <L::Service as Service<JobCall>>::Response: IntoJobResult + 'static,
        <L::Service as Service<JobCall>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<JobCall>>::Future: Send + 'static,
    {
        let routes = self
            .routes
            .into_iter()
            .map(|(id, endpoint)| {
                let route = endpoint.layer(layer.clone());
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
        <L::Service as Service<JobCall>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<JobCall>>::Future: Send + 'static,
    {
        if self.routes.is_empty() {
            panic!(
                "Adding a route_layer before any routes is a no-op. \
                 Add the routes you want the layer to apply to first."
            );
        }

        let routes = self
            .routes
            .into_iter()
            .map(|(id, endpoint)| {
                let route = endpoint.layer(layer.clone());
                (id, route)
            })
            .collect();

        JobIdRouter {
            routes,
            node: self.node,
            prev_route_id: self.prev_route_id,
        }
    }

    pub(super) fn has_routes(&self) -> bool {
        !self.routes.is_empty()
    }

    pub(super) fn call_with_context<Ctx>(
        &self,
        mut call: JobCall,
        context: Ctx,
    ) -> Result<RouteFuture<Infallible>, (JobCall, Ctx)>
    where
        Ctx: Clone + Send + Sync + 'static,
    {
        let (mut parts, body) = call.into_parts();
        let Some(route) = self.node.get(parts.job_id) else {
            return Err((JobCall::from_parts(parts, body), context));
        };

        let route = self
            .routes
            .get(&route)
            .expect("no route for id. This is a bug in axum. Please file an issue");

        let req = JobCall::from_parts(parts, body);
        Ok(route.clone().call_owned(req))
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

impl<const IS_FALLBACK: bool> Default for JobIdRouter<IS_FALLBACK> {
    fn default() -> Self {
        Self {
            routes: Default::default(),
            node: Default::default(),
            prev_route_id: RouteId(0),
        }
    }
}

impl<const IS_FALLBACK: bool> fmt::Debug for JobIdRouter<IS_FALLBACK> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PathRouter")
            .field("routes", &self.routes)
            .field("node", &self.node)
            .finish()
    }
}

impl<const IS_FALLBACK: bool> Clone for JobIdRouter<IS_FALLBACK> {
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
