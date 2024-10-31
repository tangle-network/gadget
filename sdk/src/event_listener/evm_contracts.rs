use super::EventListener;
use crate::event_listener::get_exponential_backoff;
use crate::events_watcher::evm::EvmEventHandler;
use crate::store::LocalDatabase;
use crate::{error, Error};
use alloy_contract::{ContractInstance, Event};
use alloy_network::Ethereum;
use alloy_provider::Provider;
use alloy_rpc_types::{BlockNumberOrTag, Filter};
use alloy_sol_types::SolEvent;
use std::collections::VecDeque;
use std::time::Duration;
use tokio_retry::Retry;
use tracing::{info, warn};
use uuid::Uuid;

pub trait EthereumContractBound:
    Clone
    + Send
    + Sync
    + crate::events_watcher::evm::Config<
        TH: alloy_transport::Transport + Clone + Send + Sync + 'static,
        PH: alloy_provider::Provider<<Self as crate::events_watcher::evm::Config>::TH>
                + Clone
                + Send
                + Sync
                + 'static,
    > + 'static
{
}

// Impl EthereumContractBound for any T satisfying the bounds
impl<T> EthereumContractBound for T where
    T: Clone
        + Send
        + Sync
        + crate::events_watcher::evm::Config<
            TH: alloy_transport::Transport + Clone + Send + Sync + 'static,
            PH: alloy_provider::Provider<<T as crate::events_watcher::evm::Config>::TH>
                    + Clone
                    + Send
                    + Sync
                    + 'static,
        > + 'static
{
}

pub struct EthereumHandlerWrapper<W: EvmEventHandler<T>, T: EthereumContractBound> {
    instance: W,
    chain_id: u64,
    local_db: LocalDatabase<u64>,
    should_cooldown: bool,
    enqueued_events: VecDeque<(W::Event, alloy_rpc_types::Log)>,
    _phantom: std::marker::PhantomData<T>,
}

pub trait EvmContractInstance<T: EthereumContractBound> {
    fn get_instance(&self) -> &ContractInstance<T::TH, T::PH, Ethereum>;
}

pub type EvmWatcherWrapperContext<W> = W;

#[async_trait::async_trait]
impl<T: EthereumContractBound, Watcher: Clone + EvmEventHandler<T> + EvmContractInstance<T>>
    EventListener<(Watcher::Event, alloy_rpc_types::Log), EvmWatcherWrapperContext<Watcher>>
    for EthereumHandlerWrapper<Watcher, T>
{
    async fn new(context: &EvmWatcherWrapperContext<Watcher>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let provider = context.get_instance().provider().root();
        // Add more detailed error handling and logging
        let chain_id = provider
            .get_chain_id()
            .await
            .map_err(|err| Error::Client(format!("Failed to get chain ID: {}", err)))?;

        let local_db = LocalDatabase::open(format!("./db/{}", Uuid::new_v4()));
        Ok(Self {
            chain_id,
            should_cooldown: false,
            enqueued_events: VecDeque::new(),
            local_db,
            instance: context.clone(),
            _phantom: std::marker::PhantomData,
        })
    }

    async fn next_event(&mut self) -> Option<(Watcher::Event, alloy_rpc_types::Log)> {
        if let Some(event) = self.enqueued_events.pop_front() {
            return Some(event);
        }

        if self.should_cooldown {
            tokio::time::sleep(Duration::from_millis(5000)).await;
            self.should_cooldown = false;
        }

        let contract = &self.instance;
        let step = 100;
        let target_block_number: u64 = contract
            .get_instance()
            .provider()
            .get_block_number()
            .await
            .unwrap_or_default();
        // loop {
        // Get the latest block number
        let block = self
            .local_db
            .get(&format!(
                "LAST_BLOCK_NUMBER_{}",
                contract.get_instance().address()
            ))
            .unwrap_or(0);

        let should_cooldown = block >= target_block_number;
        if should_cooldown {
            self.should_cooldown = true;
            return self.next_event().await;
        }

        let dest_block = core::cmp::min(block + step, target_block_number);

        // Query events
        let events_filter = Event::new(contract.get_instance().provider(), Filter::new())
            .address(*contract.get_instance().address())
            .from_block(BlockNumberOrTag::Number(block + 1))
            .to_block(BlockNumberOrTag::Number(dest_block))
            .event_signature(Watcher::Event::SIGNATURE_HASH);

        info!("Querying events for filter, address: {}, from_block: {}, to_block: {}, event_signature: {}", contract.get_instance().address(), block + 1, dest_block, Watcher::Event::SIGNATURE_HASH);
        match events_filter.query().await {
            Ok(events) => {
                let events = events.into_iter().collect::<VecDeque<_>>();
                self.local_db.set(
                    &format!("LAST_BLOCK_NUMBER_{}", contract.get_instance().address()),
                    dest_block,
                );

                self.local_db.set(
                    &format!("TARGET_BLOCK_{}", contract.get_instance().address()),
                    target_block_number,
                );

                if events.is_empty() {
                    self.should_cooldown = true;
                    return self.next_event().await;
                }

                self.enqueued_events = events;

                self.next_event().await
            }
            Err(e) => {
                error!(?e, %self.chain_id, "Error while querying events");
                None
            }
        }
    }

    async fn handle_event(
        &mut self,
        (event, log): (Watcher::Event, alloy_rpc_types::Log),
    ) -> Result<(), Error> {
        const MAX_RETRIES: usize = 5;
        let backoff = get_exponential_backoff::<MAX_RETRIES>();
        let handler = self.instance.clone();

        let task = async move {
            Retry::spawn(backoff, || async { handler.handle(&log, &event).await }).await
        };

        if let Err(e) = task.await {
            error!(?e, %self.chain_id, "Error while handling the event");
        }

        Ok(())
    }

    async fn execute(&mut self) -> Result<(), Error> {
        const MAX_RETRY_COUNT: usize = 10;
        let mut backoff = get_exponential_backoff::<MAX_RETRY_COUNT>();

        let mut retry_count = 0;
        match self.run_event_loop().await {
            Ok(_) => Ok(()),
            Err(e) => {
                if retry_count < MAX_RETRY_COUNT {
                    retry_count += 1;
                    tokio::time::sleep(backoff.nth(retry_count).unwrap()).await;
                    self.execute().await
                } else {
                    Err(e)
                }
            }
        }
    }
}

impl<T: EthereumContractBound, Watcher: Clone + EvmEventHandler<T> + EvmContractInstance<T>>
    EthereumHandlerWrapper<Watcher, T>
{
    async fn run_event_loop(&mut self) -> Result<(), Error> {
        while let Some(event) = self.next_event().await {
            info!(
                "Handling event of type {}",
                std::any::type_name::<Watcher::Event>()
            );
            self.handle_event(event).await?;
        }

        warn!("Event listener has stopped");
        Err(Error::Other("Event listener has stopped".to_string()))
    }
}
