use core::pin::Pin;

use blueprint_core::JobCall;
use blueprint_core::error::BoxError;
use futures_util::TryFutureExt;
use tangle_subxt::subxt_signer::bip39::core::future::Future;
use tower::{Layer, Service};

use crate::extract::BlockHash;
use crate::producer::TangleClient;

/// Automatically add all the events that happened in the current block to the job call extensions.
#[derive(Clone, Debug)]
pub struct AddBlockEvents<S> {
    pub(crate) client: TangleClient,
    pub(crate) inner: S,
}

impl<S> AddBlockEvents<S> {
    pub fn new(client: TangleClient, inner: S) -> Self {
        Self { client, inner }
    }

    pub fn inner(&self) -> &S {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S> Service<JobCall> for AddBlockEvents<S>
where
    S: Service<JobCall> + Send + Clone + 'static,
    S::Future: Send + 'static,
    S::Error: Into<BoxError> + Send + 'static,
    S::Response: 'static,
{
    type Response = S::Response;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, job_call: JobCall) -> Self::Future {
        let client = self.client.clone();
        let clone = self.inner.clone();
        // take the service that was ready
        let mut inner = std::mem::replace(&mut self.inner, clone);
        Box::pin(async move {
            let (mut parts, body) = job_call.into_parts();
            let block_hash = BlockHash::try_from(&mut parts).map_err(BoxError::from)?;
            let block = client
                .blocks()
                .at(block_hash.0)
                .map_err(BoxError::from)
                .await?;
            let events = block.events().map_err(BoxError::from).await?;
            // Add the block events to the job call extensions.
            parts.extensions.insert(events);

            let job_call = JobCall::from_parts(parts, body);
            inner.call(job_call).await.map_err(Into::into)
        })
    }
}

// A layer for adding block events to the job call extensions.
#[derive(Clone, Debug)]
pub struct AddBlockEventsLayer(pub TangleClient);

impl AddBlockEventsLayer {
    #[must_use]
    pub fn new(client: TangleClient) -> Self {
        Self(client)
    }
}

impl<S> Layer<S> for AddBlockEventsLayer {
    type Service = AddBlockEvents<S>;

    fn layer(&self, service: S) -> Self::Service {
        AddBlockEvents::new(self.0.clone(), service)
    }
}
