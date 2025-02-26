pub mod block_events;

use crate::extract;
use blueprint_core::{JobCall, JobResult};
use core::pin::Pin;
use pin_project_lite::pin_project;
use std::task::{Context, Poll, ready};
use tangle_subxt::subxt_signer::bip39::core::future::Future;
use tower::{Layer, Service};

#[derive(Copy, Clone, Debug)]
pub struct TangleSubmissionService<S> {
    service: S,
}

pin_project! {
    /// Response future of [`TangleSubmissionService`].
    #[derive(Debug)]
    pub struct TangleSubmissionFuture<F> {
        #[pin]
        kind: Kind<F>
    }
}

impl<F> TangleSubmissionFuture<F> {
    fn valid(future: F, call_id: extract::CallId, service_id: extract::ServiceId) -> Self {
        Self {
            kind: Kind::Valid {
                future,
                call_id,
                service_id,
            },
        }
    }

    fn invalid() -> Self {
        Self {
            kind: Kind::Invalid,
        }
    }
}

pin_project! {
    #[project = KindProj]
    #[derive(Debug)]
    enum Kind<F> {
        Valid {
            #[pin]
            future: F,
            call_id: extract::CallId,
            service_id: extract::ServiceId,
        },
        Invalid,
    }
}

impl<F, B, E> Future for TangleSubmissionFuture<F>
where
    F: Future<Output = Result<Option<JobResult<B>>, E>>,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().kind.project() {
            KindProj::Valid {
                future,
                call_id,
                service_id,
            } => {
                let result = ready!(future.poll(cx)?);
                match result {
                    Some(mut result) => {
                        let metadata = result.metadata_mut();
                        metadata.insert(extract::CallId::METADATA_KEY, call_id.0);
                        metadata.insert(extract::ServiceId::METADATA_KEY, service_id.0);
                        Poll::Ready(Ok(Some(result)))
                    }
                    None => Poll::Ready(Ok(None)),
                }
            }
            KindProj::Invalid => {
                // Malformed call, ignore
                Poll::Ready(Ok(None))
            }
        }
    }
}

impl<S> Service<JobCall> for TangleSubmissionService<S>
where
    S: Service<JobCall, Response = Option<JobResult>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = TangleSubmissionFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, call: JobCall) -> Self::Future {
        let (mut parts, body) = call.into_parts();
        let Ok(call_id) = extract::CallId::try_from(&mut parts) else {
            return TangleSubmissionFuture::invalid();
        };
        let Ok(service_id) = extract::ServiceId::try_from(&mut parts) else {
            return TangleSubmissionFuture::invalid();
        };

        let call = JobCall::from_parts(parts, body);
        TangleSubmissionFuture::valid(self.service.call(call), call_id, service_id)
    }
}

/// A layer to make [`JobResult`]s visible to a [`TangleConsumer`]
///
/// [`TangleConsumer`]: crate::consumer::TangleConsumer
#[derive(Copy, Clone, Debug)]
pub struct TangleLayer;

impl<S> Layer<S> for TangleLayer {
    type Service = TangleSubmissionService<S>;

    fn layer(&self, service: S) -> Self::Service {
        TangleSubmissionService { service }
    }
}
