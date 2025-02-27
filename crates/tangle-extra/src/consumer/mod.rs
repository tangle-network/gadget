use crate::extract;
use crate::extract::{InvalidCallId, InvalidServiceId};
use crate::producer::TangleConfig;
use crate::tracing;
use blueprint_core::{BoxError, JobResult};
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
        let metadata = item.metadata();
        let (Some(call_id_raw), Some(service_id_raw)) = (
            metadata.get(extract::CallId::METADATA_KEY),
            metadata.get(extract::ServiceId::METADATA_KEY),
        ) else {
            // Not a tangle job result
            return Ok(());
        };

        let call_id: CallId = call_id_raw.try_into().map_err(|_| InvalidCallId)?;
        let service_id: ServiceId = service_id_raw.try_into().map_err(|_| InvalidServiceId)?;
        let result: SubmitResult = SubmitResult::decode(&mut (&**item.body()))?;

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
                    let fut = send(consumer.client.clone(), consumer.signer.clone(), tx);
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

/// Send a transaction to the Tangle network.
///
/// # Errors
///
/// Returns a [`subxt::Error`] if the transaction fails.
async fn send<T, S, X>(
    client: Arc<subxt::OnlineClient<T>>,
    signer: Arc<S>,
    xt: X,
) -> Result<subxt::blocks::ExtrinsicEvents<T>, subxt::Error>
where
    T: subxt::Config,
    S: subxt::tx::Signer<T>,
    X: subxt::tx::Payload,
    <T::ExtrinsicParams as subxt::config::ExtrinsicParams<T>>::Params: Default,
{
    #[cfg(feature = "tracing")]
    if let Some(details) = xt.validation_details() {
        tracing::debug!("Calling {}.{}", details.pallet_name, details.call_name);
    }

    tracing::debug!("Waiting for the transaction to be included in a finalized block");
    let progress = client
        .tx()
        .sign_and_submit_then_watch_default(&xt, &*signer)
        .await?;

    #[cfg(not(test))]
    {
        tracing::debug!("Waiting for finalized success ...");
        let result = progress.wait_for_finalized_success().await?;
        tracing::debug!(
            "Transaction with hash: {:?} has been finalized",
            result.extrinsic_hash()
        );
        Ok(result)
    }
    #[cfg(test)]
    {
        use gadget_utils_tangle::tx_progress::TxProgressExt;

        // In tests, we don't wait for the transaction to be finalized.
        // This is because the test environment we will be using instant sealing.
        // Instead, we just wait for the transaction to be included in a block.
        tracing::debug!("Waiting for in block success ...");
        let result = progress.wait_for_in_block_success().await?;
        tracing::debug!(
            "Transaction with hash: {:?} has been included in a block",
            result.extrinsic_hash()
        );
        Ok(result)
    }
}
