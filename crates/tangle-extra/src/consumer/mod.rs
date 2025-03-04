use crate::extract;
use crate::extract::{InvalidCallId, InvalidServiceId};
use crate::producer::TangleConfig;
use blueprint_core::JobResult;
use blueprint_core::error::BoxError;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_util::Sink;
use std::collections::VecDeque;
use std::sync::Arc;
use tangle_subxt::parity_scale_codec::Decode;
use tangle_subxt::subxt;
use tangle_subxt::subxt::OnlineClient;
use tangle_subxt::subxt_core::tx::signer::Signer;
use tangle_subxt::tangle_testnet_runtime::api;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::submit_result::{
    CallId, Result as SubmitResult, ServiceId,
};

enum State {
    WaitingForResult,
    ProcessingBlock(
        Pin<
            Box<
                dyn Future<
                        Output = Result<subxt::blocks::ExtrinsicEvents<TangleConfig>, subxt::Error>,
                    > + Send,
            >,
        >,
    ),
}

impl State {
    fn is_waiting(&self) -> bool {
        matches!(self, State::WaitingForResult)
    }
}

struct DerivedJobResult {
    call_id: CallId,
    service_id: ServiceId,
    result: SubmitResult,
}

/// A consumer of Tangle [`JobResult`]s
pub struct TangleConsumer<S> {
    client: Arc<OnlineClient<TangleConfig>>,
    signer: Arc<S>,
    buffer: VecDeque<DerivedJobResult>,
    state: State,
}

impl<S> TangleConsumer<S>
where
    S: Signer<TangleConfig> + Send + Sync + Unpin + 'static,
{
    /// Create a new [`TangleConsumer`]
    pub fn new(client: OnlineClient<TangleConfig>, signer: S) -> Self {
        Self {
            client: Arc::new(client),
            signer: Arc::new(signer),
            buffer: VecDeque::new(),
            state: State::WaitingForResult,
        }
    }
}

impl<S> Sink<JobResult> for TangleConsumer<S>
where
    S: Signer<TangleConfig> + Send + Sync + Unpin + 'static,
{
    type Error = BoxError;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: JobResult) -> Result<(), Self::Error> {
        let JobResult::Ok { head, body } = &item else {
            // We don't care about errors here
            return Ok(());
        };

        let (Some(call_id_raw), Some(service_id_raw)) = (
            head.metadata.get(extract::CallId::METADATA_KEY),
            head.metadata.get(extract::ServiceId::METADATA_KEY),
        ) else {
            // Not a tangle job result
            return Ok(());
        };

        let call_id: CallId = call_id_raw.try_into().map_err(|_| InvalidCallId)?;
        let service_id: ServiceId = service_id_raw.try_into().map_err(|_| InvalidServiceId)?;
        let result: SubmitResult = SubmitResult::decode(&mut (&**body))?;

        self.get_mut().buffer.push_back(DerivedJobResult {
            call_id,
            service_id,
            result,
        });
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.buffer.is_empty() && self.state.is_waiting() {
            return Poll::Ready(Ok(()));
        }

        let consumer = self.get_mut();
        loop {
            match consumer.state {
                State::WaitingForResult => {
                    let Some(DerivedJobResult {
                        call_id,
                        service_id,
                        result,
                    }) = consumer.buffer.pop_front()
                    else {
                        return Poll::Ready(Ok(()));
                    };

                    let tx = api::tx()
                        .services()
                        .submit_result(service_id, call_id, result);
                    let fut =
                        crate::util::send(consumer.client.clone(), consumer.signer.clone(), tx);
                    consumer.state = State::ProcessingBlock(Box::pin(fut));
                    continue;
                }
                State::ProcessingBlock(ref mut future) => match future.as_mut().poll(cx) {
                    Poll::Ready(Ok(_extrinsic_events)) => {
                        consumer.state = State::WaitingForResult;
                        continue;
                    }
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                    Poll::Pending => return Poll::Pending,
                },
            }
        }
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.buffer.is_empty() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
}
