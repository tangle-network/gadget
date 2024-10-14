use crate::clients::tangle::runtime::{TangleClient, TangleConfig};
use crate::event_listener::get_exponential_backoff;
use crate::events_watcher::evm::{Config as ConfigT, EvmEventHandler};
use crate::events_watcher::substrate::EventHandlerFor;
use crate::store::LocalDatabase;
use crate::{error, trace, warn, Error};
use alloy_network::ReceiptResponse;
use alloy_provider::Provider;
use alloy_rpc_types::{BlockNumberOrTag, Filter};
use async_trait::async_trait;
use std::collections::HashMap;
use std::iter::Take;
use std::sync::Arc;
use std::time::Duration;
use subxt::backend::StreamOfResults;
use subxt::OnlineClient;
use subxt_core::events::StaticEvent;
use tangle_subxt::tangle_testnet_runtime::api::services::events::{
    job_called, JobCalled, JobResultSubmitted,
};
use tokio::sync::Mutex;
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;

use super::EventListener;

pub struct EthereumWatcherWrapper<W: EvmEventHandler<C>, C: ConfigT> {
    handler: Arc<W>,
    contract: W::Contract,
    chain_id: u64,
    target_block_number: Option<u64>,
    dest_block: Option<u64>,
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
        Ok(Self {
            chain_id,
            target_block_number: None,
            dest_block: None,
            local_db: LocalDatabase::open("./db"),
            handler: context.1.clone(),
            contract: context.0.clone(),
            _phantom: std::marker::PhantomData,
        })
    }

    async fn next_event(&mut self) -> Option<Vec<(Watcher::Event, alloy_rpc_types::Log)>> {
        let contract = &self.contract;
        let step = 100;

        // we only query this once, at the start of the events watcher.
        // then we will update it later once we fully synced.
        self.target_block_number = Some(contract.provider().get_block_number().await.ok()?);

        self.local_db.set(
            &format!("TARGET_BLOCK_NUMBER_{:?}", contract.address()),
            self.target_block_number.expect("qed"),
        );

        let deployed_at = contract
            .provider()
            .get_transaction_receipt(Watcher::GENESIS_TX_HASH)
            .await
            .ok()?
            .map(|receipt| receipt.block_number().unwrap_or_default())
            .unwrap_or_default();

        loop {
            let block = self
                .local_db
                .get(&format!("LAST_BLOCK_NUMBER_{}", contract.address()))
                .unwrap_or(deployed_at);
            self.dest_block = Some(core::cmp::min(
                block + step,
                self.target_block_number.expect("qed"),
            ));

            let events_filter = contract.event::<Watcher::Event>(
                Filter::new()
                    .from_block(BlockNumberOrTag::Number(block + 1))
                    .to_block(BlockNumberOrTag::Number(self.dest_block.expect("qed"))),
            );

            let events = events_filter.query().await.ok()?;
            let events: Vec<_> = events.into_iter().collect();
            if events.is_empty() {
                continue;
            }

            return Some(events);
        }
    }

    async fn handle_event(
        &mut self,
        events: Vec<(Watcher::Event, alloy_rpc_types::Log)>,
    ) -> Result<(), Error> {
        const MAX_RETRIES: usize = 5;
        let mut tasks = vec![];
        let current_block_number = events[0].1.block_number;
        for (event, log) in &events {
            let backoff = get_exponential_backoff::<MAX_RETRIES>();
            let handler = self.handler.clone();
            let task = async move {
                Retry::spawn(backoff, || async { handler.handle(log, event).await }).await
            };

            tasks.push(task);
        }

        let result = futures::future::join_all(tasks).await;
        // this event will be marked as handled if at least one handler succeeded.
        // this because, for the failed events, we already tried to handle them
        // many times (at this point), and there is no point in trying again.
        let mark_as_handled = result.iter().any(Result::is_ok);
        // also, for all the failed event handlers, we should print what went
        // wrong.
        for r in &result {
            if let Err(e) = r {
                error!(?e, %self.chain_id, "Error while handling the event");
            }
        }

        if mark_as_handled {
            self.local_db.set(
                &format!("LAST_BLOCK_NUMBER_{}", self.contract.address()),
                current_block_number.unwrap_or_default(),
            );
        } else {
            error!(
                "{} | Error while handling event, all handlers failed.",
                self.chain_id
            );
            warn!("{} | Restarting event watcher ...", self.chain_id);
            // this a transient error, so we will retry again.
            return Ok(());
        }

        let dest_block = self.dest_block.expect("qed");

        // move the block pointer to the destination block
        self.local_db.set(
            &format!("LAST_BLOCK_NUMBER_{}", self.contract.address()),
            dest_block,
        );
        // if we fully synced, we can update the target block number
        let should_cooldown = dest_block == self.target_block_number.expect("qed");
        if should_cooldown {
            let duration = Duration::from_secs(10);
            trace!("Cooldown a bit for {}ms", duration.as_millis());
            tokio::time::sleep(duration).await;
            // update the latest block number
            self.target_block_number = Some(
                self.contract
                    .provider()
                    .get_block_number()
                    .await
                    .map_err(Into::<crate::events_watcher::Error>::into)?,
            );
            self.local_db.set(
                &format!("TARGET_BLOCK_NUMBER_{}", self.contract.address()),
                self.target_block_number.expect("qed"),
            );
        }

        Ok(())
    }

    async fn execute(&mut self) -> Result<(), Error> {
        const MAX_RETRY_COUNT: usize = 10;
        let mut backoff = get_exponential_backoff::<MAX_RETRY_COUNT>();

        let mut retry_count = 0;
        loop {
            match self.run_event_loop().await {
                Ok(_) => continue,
                Err(e) => {
                    if retry_count >= MAX_RETRY_COUNT {
                        break Err(e);
                    }
                    retry_count += 1;
                    tokio::time::sleep(backoff.nth(retry_count).unwrap()).await;
                }
            }
        }
    }
}

impl<Config: ConfigT, Watcher: EvmEventHandler<Config>> EthereumWatcherWrapper<Watcher, Config> {
    async fn run_event_loop(&mut self) -> Result<(), Error> {
        while let Some(events) = self.next_event().await {
            self.handle_event(events).await?;
        }

        crate::warn!("Event listener has stopped");
        Err(Error::Other("Event listener has stopped".to_string()))
    }
}
