use alloc::collections::VecDeque;
use blueprint_sdk::extensions::Extensions;
use blueprint_sdk::job_call::Parts;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;
use tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;

use blueprint_sdk::metadata::{MetadataMap, MetadataValue};
use blueprint_sdk::JobCall;
use futures_core::Stream;
use futures_util::{StreamExt, TryFutureExt};
use tangle_subxt::parity_scale_codec::Encode;
use tangle_subxt::subxt::backend::StreamOfResults;
use tangle_subxt::subxt::blocks::{Block, BlocksClient};
use tangle_subxt::subxt::config::Header;
use tangle_subxt::subxt::{self, OnlineClient, PolkadotConfig};

use super::extract;

pub type TangleConfig = PolkadotConfig;
pub type TangleClient = OnlineClient<TangleConfig>;
pub type TangleBlock = Block<TangleConfig, TangleClient>;
type BlockStream<T> = StreamOfResults<T>;

pub struct TangleProducer<Finalization = ()> {
    blocks_client: BlocksClient<TangleConfig, TangleClient>,
    s: Finalization,
    buffer: VecDeque<JobCall>,
}

impl TangleProducer<()> {
    pub async fn finalized_blocks(
        client: TangleClient,
    ) -> Result<TangleProducer<BlockStream<TangleBlock>>, subxt::Error> {
        let blocks_client = BlocksClient::new(client);
        let s = blocks_client.subscribe_finalized().await?;
        Ok(TangleProducer {
            blocks_client,
            s,
            buffer: VecDeque::new(),
        })
    }

    pub async fn best_blocks(
        client: TangleClient,
    ) -> Result<TangleProducer<BlockStream<TangleBlock>>, subxt::Error> {
        let blocks_client = BlocksClient::new(client);
        let s = blocks_client.subscribe_best().await?;
        Ok(TangleProducer {
            blocks_client,
            s,
            buffer: VecDeque::new(),
        })
    }

    pub fn blocks_client(&self) -> &BlocksClient<TangleConfig, TangleClient> {
        &self.blocks_client
    }
}

impl<Finalization> Stream for TangleProducer<Finalization>
where
    Finalization: Stream<Item = Result<TangleBlock, subxt::Error>> + Unpin,
{
    type Item = Result<JobCall, subxt::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let block = match self.s.poll_next_unpin(cx) {
            Poll::Ready(Some(result)) => match result {
                Ok(block) => block,
                Err(e) => return Poll::Ready(Some(Err(e))),
            },
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => return Poll::Pending,
        };

        let mut f = core::pin::pin!(block_to_job_calls(block));
        let job_calls = match f.try_poll_unpin(cx) {
            futures_core::task::Poll::Ready(t) => t?,
            futures_core::task::Poll::Pending => return Poll::Pending,
        };
        self.buffer.extend(job_calls);

        if let Some(job_call) = self.buffer.pop_front() {
            Poll::Ready(Some(Ok(job_call)))
        } else {
            Poll::Pending
        }
    }
}

/// Converts Tangle Blocks into Job Calls
async fn block_to_job_calls(block: TangleBlock) -> Result<Vec<JobCall>, subxt::Error> {
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
                // metadata.insert(extract::CallerAccountId::METADATA_KEY, c.caller);
                metadata.insert(extract::ServiceId::METADATA_KEY, c.service_id);
                let parts = Parts::new(c.job.into())
                    .with_metadata(metadata)
                    .with_extensions(extensions.clone());
                JobCall::from_parts(parts, c.args.encode().into())
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
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
