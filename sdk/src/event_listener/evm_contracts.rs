use super::EventListener;
use crate::event_listener::get_exponential_backoff;
use crate::events_watcher::evm::{Config as ConfigT, EvmEventHandler};
use crate::store::LocalDatabase;
use crate::{error, trace, Error};
use alloy_contract::Event;
use alloy_provider::Provider;
use alloy_rpc_types::{BlockNumberOrTag, Filter};
use alloy_sol_types::SolEvent;
use std::sync::Arc;
use std::time::Duration;
use tokio_retry::Retry;
use uuid::Uuid;

pub struct EthereumWatcherWrapper<W: EvmEventHandler<C>, C: ConfigT> {
    handler: Arc<W>,
    contract: W::Contract,
    chain_id: u64,
    local_db: LocalDatabase<u64>,
    _phantom: std::marker::PhantomData<C>,
}

pub type EvmWatcherWrapperContext<W, Config> = (<W as EvmEventHandler<Config>>::Contract, Arc<W>);

#[async_trait::async_trait]
impl<Config: ConfigT, Watcher: EvmEventHandler<Config>>
    EventListener<
        Vec<(Watcher::Event, alloy_rpc_types::Log)>,
        EvmWatcherWrapperContext<Watcher, Config>,
    > for EthereumWatcherWrapper<Watcher, Config>
{
    async fn new(context: &EvmWatcherWrapperContext<Watcher, Config>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        println!(
            "event_listener/mod.rs | Initializing event handler for {}",
            std::any::type_name::<Watcher>()
        );
        let provider = context.0.provider().root();
        println!("event_listener/mod.rs | Root provider initialized");
        // Add more detailed error handling and logging
        let chain_id: u64 = match provider.get_chain_id().await {
            Ok(id) => {
                println!(
                    "event_listener/mod.rs | Successfully retrieved chain ID: {}",
                    id
                );
                id
            }
            Err(err) => {
                println!("event_listener/mod.rs | Error getting chain ID: {:?}", err);
                return Err(Error::Client(format!("Failed to get chain ID: {}", err)));
            }
        };

        println!("event_listener/mod.rs | Chain ID: {}", chain_id);
        let local_db = LocalDatabase::open(&format!("./db/{}", Uuid::new_v4()));
        Ok(Self {
            chain_id,
            local_db,
            handler: context.1.clone(),
            contract: context.0.clone(),
            _phantom: std::marker::PhantomData,
        })
    }

    async fn next_event(&mut self) -> Option<Vec<(Watcher::Event, alloy_rpc_types::Log)>> {
        let contract = &self.contract;
        let step = 100;
        let mut target_block_number: u64 = contract
            .provider()
            .get_block_number()
            .await
            .unwrap_or_default();
        loop {
            // Get the latest block number
            let block = self
                .local_db
                .get(&format!("LAST_BLOCK_NUMBER_{}", contract.address()))
                .unwrap_or(0);

            let should_cooldown = block >= target_block_number;
            if should_cooldown {
                let duration = Duration::from_secs(10);
                trace!("Cooldown a bit for {}ms", duration.as_millis());
                tokio::time::sleep(duration).await;
                // update the latest block number
                target_block_number = contract.provider().get_block_number().await.unwrap();
            }

            let dest_block = core::cmp::min(block + step, target_block_number);
            println!("evm_contract.rs | Querying from block {block} to {dest_block}");

            // Query events
            let events_filter = Event::new(contract.provider(), Filter::new())
                .address(*contract.address())
                .from_block(BlockNumberOrTag::Number(block + 1))
                .to_block(BlockNumberOrTag::Number(dest_block))
                .event_signature(Watcher::Event::SIGNATURE_HASH);

            println!("evm_contracts.rs | Querying events for filter, address: {}, from_block: {}, to_block: {}, event_signature: {}", contract.address(), block + 1, dest_block, Watcher::Event::SIGNATURE_HASH);
            match events_filter.query().await {
                Ok(events) => {
                    println!("evm_contracts.rs | Found {} events", events.len());

                    self.local_db.set(
                        &format!("LAST_BLOCK_NUMBER_{}", contract.address()),
                        dest_block,
                    );

                    self.local_db.set(
                        &format!("TARGET_BLOCK_{}", contract.address()),
                        target_block_number,
                    );

                    return Some(events);
                }
                Err(e) => {
                    error!(?e, %self.chain_id, "Error while querying events");
                    return None;
                }
            }
        }
    }

    async fn handle_event(
        &mut self,
        events: Vec<(Watcher::Event, alloy_rpc_types::Log)>,
    ) -> Result<(), Error> {
        const MAX_RETRIES: usize = 5;
        let mut tasks = vec![];
        println!(
            "evm_contracts.rs | handling event {} with {} logs",
            std::any::type_name::<Watcher>(),
            events.len()
        );
        for (event, log) in &events {
            let backoff = get_exponential_backoff::<MAX_RETRIES>();
            let handler = self.handler.clone();
            println!("evm_contracts.rs | event log {:?}", log);
            let task = async move {
                Retry::spawn(backoff, || async { handler.handle(log, event).await }).await
            };

            tasks.push(task);
        }

        let result = futures::future::join_all(tasks).await;
        // Log the errors for all the failed tasks
        for r in &result {
            if let Err(e) = r {
                error!(?e, %self.chain_id, "Error while handling the event");
            }
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

impl<Config: ConfigT, Watcher: EvmEventHandler<Config>> EthereumWatcherWrapper<Watcher, Config> {
    async fn run_event_loop(&mut self) -> Result<(), Error> {
        println!(
            "evm_contracts.rs | Running event loop for {}",
            std::any::type_name::<Watcher>()
        );
        while let Some(events) = self.next_event().await {
            if events.is_empty() {
                continue;
            }

            println!("evm_contracts.rs | Handling {} events", events.len());
            self.handle_event(events).await?;
        }

        crate::warn!("Event listener has stopped");
        Err(Error::Other("Event listener has stopped".to_string()))
    }
}
