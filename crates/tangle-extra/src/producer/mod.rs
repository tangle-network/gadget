use crate::extract;
use alloc::collections::VecDeque;
use blueprint_core::JobCall;
use blueprint_core::extensions::Extensions;
use blueprint_core::job_call::Parts;
use blueprint_core::metadata::{MetadataMap, MetadataValue};
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use futures_core::Stream;
use std::sync::Mutex;
use tangle_subxt::parity_scale_codec::Encode;
use tangle_subxt::subxt::blocks::{Block, BlocksClient};
use tangle_subxt::subxt::config::Header;
use tangle_subxt::subxt::{self, OnlineClient, PolkadotConfig};
use tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;

pub type TangleConfig = PolkadotConfig;
pub type TangleClient = OnlineClient<TangleConfig>;
pub type TangleBlock = Block<TangleConfig, TangleClient>;

struct ProducerState {
    block_stream:
        Pin<Box<dyn Stream<Item = Result<TangleBlock, subxt::Error>> + Send + Unpin + 'static>>,
    buffer: VecDeque<JobCall>,
    block_in_progress:
        Option<Pin<Box<dyn Future<Output = Result<Vec<JobCall>, subxt::Error>> + Send>>>,
}

impl ProducerState {
    fn new(
        block_stream: impl Stream<Item = Result<TangleBlock, subxt::Error>> + Send + Unpin + 'static,
    ) -> Self {
        Self {
            block_stream: Box::pin(block_stream),
            buffer: VecDeque::new(),
            block_in_progress: None,
        }
    }
}

/// A producer of Tangle [`JobCall`]s
pub struct TangleProducer {
    blocks_client: BlocksClient<TangleConfig, TangleClient>,
    state: Mutex<ProducerState>,
}

impl TangleProducer {
    /// Create a TangleProducer that yields job calls from finalized blocks.
    pub async fn finalized_blocks(client: TangleClient) -> Result<Self, subxt::Error> {
        let blocks_client = BlocksClient::new(client.clone());
        let stream = blocks_client.subscribe_finalized().await?;
        let state = ProducerState::new(stream);
        Ok(Self {
            blocks_client,
            state: Mutex::new(state),
        })
    }

    /// Create a TangleProducer that yields job calls from best blocks.
    pub async fn best_blocks(client: TangleClient) -> Result<Self, subxt::Error> {
        let blocks_client = BlocksClient::new(client.clone());
        let stream = blocks_client.subscribe_best().await?;
        let state = ProducerState::new(stream);
        Ok(Self {
            blocks_client,
            state: Mutex::new(state),
        })
    }

    /// Returns a reference to the blocks client.
    pub fn blocks_client(&self) -> &BlocksClient<TangleConfig, TangleClient> {
        &self.blocks_client
    }
}

impl Stream for TangleProducer {
    type Item = Result<JobCall, subxt::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut state = self.state.lock().unwrap();

        loop {
            match state.block_in_progress.as_mut() {
                Some(proc_future) => match proc_future.as_mut().poll(cx) {
                    Poll::Ready(Ok(job_calls)) => {
                        state.buffer.extend(job_calls);
                        state.block_in_progress = None;
                        if let Some(job) = state.buffer.pop_front() {
                            return Poll::Ready(Some(Ok(job)));
                        }

                        continue;
                    }
                    Poll::Ready(Err(e)) => {
                        state.block_in_progress = None;
                        return Poll::Ready(Some(Err(e)));
                    }
                    Poll::Pending => {
                        if let Some(job) = state.buffer.pop_front() {
                            return Poll::Ready(Some(Ok(job)));
                        } else {
                            return Poll::Pending;
                        }
                    }
                },
                None => match state.block_stream.as_mut().poll_next(cx) {
                    Poll::Ready(Some(Ok(block))) => {
                        let fut = block_to_job_calls(block);
                        state.block_in_progress = Some(Box::pin(fut)
                            as Pin<
                                Box<dyn Future<Output = Result<Vec<JobCall>, subxt::Error>> + Send>,
                            >);
                        continue;
                    }
                    Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
                    Poll::Ready(None) => return Poll::Ready(None),
                    Poll::Pending => {
                        if let Some(job) = state.buffer.pop_front() {
                            return Poll::Ready(Some(Ok(job)));
                        } else {
                            return Poll::Pending;
                        }
                    }
                },
            }
        }
    }
}

/// Converts Tangle Blocks into Job Calls
async fn block_to_job_calls(block: TangleBlock) -> Result<Vec<JobCall>, subxt::Error> {
    blueprint_core::trace!("Processing block #{}", block.header().number);
    let header = block.header();
    let metadata = block_header_to_job_metadata(header);
    let mut extensions = Extensions::new();
    let events = block.events().await?;

    extensions.insert(events.clone());

    let job_calls = events
        .find::<JobCalled>()
        .map(|c| {
            c.map(|c| {
                let mut metadata = metadata.clone();
                metadata.insert(extract::CallId::METADATA_KEY, c.call_id);
                metadata.insert(extract::ServiceId::METADATA_KEY, c.service_id);
                metadata.insert(extract::Caller::METADATA_KEY, c.caller.0);
                let parts = Parts::new(c.job)
                    .with_metadata(metadata)
                    .with_extensions(extensions.clone());
                JobCall::from_parts(parts, c.args.encode().into())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    if job_calls.is_empty() {
        blueprint_core::trace!("No job calls in block #{}", header.number);
    }

    Ok(job_calls)
}

fn block_header_to_job_metadata(
    header: &<TangleConfig as subxt::Config>::Header,
) -> MetadataMap<MetadataValue> {
    let mut metadata = MetadataMap::new();
    metadata.insert(extract::BlockNumber::METADATA_KEY, header.number);
    metadata.insert(
        extract::BlockHash::METADATA_KEY,
        header.hash().to_fixed_bytes(),
    );
    metadata
}
