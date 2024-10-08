use crate::clients::tangle::runtime::{TangleClient, TangleConfig};
use crate::events_watcher::evm::{Config as ConfigT, EvmEventHandler};
use crate::events_watcher::substrate::EventHandlerFor;
use crate::store::LocalDatabase;
use crate::{error, trace, warn, Error};
use alloy_network::Ethereum;
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

pub mod periodic;

/// The [`EventListener`] trait defines the interface for event listeners.
#[async_trait]
pub trait EventListener<T: Send + Sync + 'static, Ctx: Send + Sync + 'static>:
    Send + Sync + 'static
{
    async fn new(context: &Ctx) -> Result<Self, Error>
    where
        Self: Sized;

    /// Obtains the next event to be processed by the event listener.
    async fn next_event(&mut self) -> Option<T>;

    /// Logic for handling received events
    async fn handle_event(&mut self, event: T) -> Result<(), Error>;

    /// The event loop for the event listener
    async fn execute(&mut self) -> Result<(), Error> {
        while let Some(event) = self.next_event().await {
            if let Err(err) = self.handle_event(event).await {
                crate::error!("Error handling event: {err}");
            }
        }

        crate::warn!("Event listener has stopped");
        Err(Error::Other("Event listener has stopped".to_string()))
    }
}

pub struct GenericEventListener<
    Event: Send + Sync + 'static,
    Ctx: Send + Sync + 'static,
    Ext: Send + Sync + 'static = (),
> {
    listener: Box<dyn EventListener<Event, Ctx>>,
    _pd: std::marker::PhantomData<Ext>,
}

#[async_trait]
impl<Event: Send + Sync + 'static, Ctx: Send + Sync + 'static, Ext: Send + Sync + 'static>
    EventListener<Event, Ctx> for GenericEventListener<Event, Ctx, Ext>
{
    async fn new(_ctx: &Ctx) -> Result<Self, Error>
    where
        Self: Sized,
    {
        unreachable!(
            "This function should not be called directly. This is for internal dev use only"
        )
    }
    async fn next_event(&mut self) -> Option<Event> {
        self.listener.next_event().await
    }

    async fn handle_event(&mut self, event: Event) -> Result<(), Error> {
        self.listener.handle_event(event).await
    }
}

impl<Event: Send + Sync + 'static, Ctx: Send + Sync + 'static, Ext: Send + Sync + 'static>
    GenericEventListener<Event, Ctx, Ext>
{
    pub fn new<T: EventListener<Event, Ctx>>(listener: T) -> Self {
        Self {
            listener: Box::new(listener),
            _pd: std::marker::PhantomData,
        }
    }
}

pub struct EthereumWatcherWrapper<W: EvmEventHandler<C>, C: ConfigT<N = Ethereum>> {
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
impl<Config: ConfigT<N = Ethereum>, Watcher: EvmEventHandler<Config>>
    EventListener<
        Vec<(Watcher::Event, alloy_rpc_types::Log)>,
        EvmWatcherWrapperContext<Watcher, Config>,
    > for EthereumWatcherWrapper<Watcher, Config>
{
    async fn new(context: &EvmWatcherWrapperContext<Watcher, Config>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let chain_id: u64 = context
            .0
            .provider()
            .root()
            .get_chain_id()
            .await
            .map_err(|err| Error::Client(err.to_string()))?;

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
            let contract = self.contract.clone();
            let task = async move {
                Retry::spawn(backoff, || async {
                    handler.handle(&contract, log, event).await
                })
                .await
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

        self.handler.init().await;

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

impl<Config: ConfigT<N = Ethereum>, Watcher: EvmEventHandler<Config>>
    EthereumWatcherWrapper<Watcher, Config>
{
    async fn run_event_loop(&mut self) -> Result<(), Error> {
        while let Some(events) = self.next_event().await {
            self.handle_event(events).await?;
        }

        crate::warn!("Event listener has stopped");
        Err(Error::Other("Event listener has stopped".to_string()))
    }
}

pub trait HasServiceAndJobId: StaticEvent + Send + Sync + 'static {
    fn service_id(&self) -> u64;
    fn job_id(&self) -> u8;
    fn id_of_call(&self) -> job_called::CallId;
}

impl HasServiceAndJobId for JobCalled {
    fn service_id(&self) -> u64 {
        self.service_id
    }

    fn job_id(&self) -> u8 {
        self.job
    }

    fn id_of_call(&self) -> job_called::CallId {
        self.call_id
    }
}

impl HasServiceAndJobId for JobResultSubmitted {
    fn service_id(&self) -> u64 {
        self.service_id
    }

    fn job_id(&self) -> u8 {
        self.job
    }

    fn id_of_call(&self) -> job_called::CallId {
        self.call_id
    }
}

fn get_exponential_backoff<const N: usize>() -> Take<ExponentialBackoff> {
    ExponentialBackoff::from_millis(2).factor(1000).take(N)
}

pub struct SubstrateWatcherWrapper<Evt: Send + Sync + 'static> {
    handlers: Vec<EventHandlerFor<TangleConfig, Evt>>,
    client: OnlineClient<TangleConfig>,
    current_block: Option<u32>,
    listener:
        Mutex<StreamOfResults<subxt::blocks::Block<TangleConfig, OnlineClient<TangleConfig>>>>,
}

pub type SubstrateWatcherWrapperContext<Evt> = (TangleClient, EventHandlerFor<TangleConfig, Evt>);
#[async_trait]
impl<Evt: Send + Sync + HasServiceAndJobId + 'static>
    EventListener<HashMap<usize, Vec<Evt>>, SubstrateWatcherWrapperContext<Evt>>
    for SubstrateWatcherWrapper<Evt>
{
    async fn new(ctx: &SubstrateWatcherWrapperContext<Evt>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let (client, handlers) = ctx;
        let listener = Mutex::new(client.blocks().subscribe_finalized().await?);
        Ok(Self {
            listener,
            client: client.clone(),
            current_block: None,
            handlers: vec![handlers.clone()],
        })
    }

    async fn next_event(&mut self) -> Option<HashMap<usize, Vec<Evt>>> {
        loop {
            let next_block = self.listener.get_mut().next().await?.ok()?;
            let block_id = next_block.number();
            self.current_block = Some(block_id);
            let events = next_block.events().await.ok()?;
            let mut actionable_events = HashMap::new();
            for (idx, handler) in self.handlers.iter().enumerate() {
                for _evt in events.iter().flatten() {
                    crate::info!(
                        "Event found || required: sid={}, jid={}",
                        handler.service_id(),
                        handler.job_id()
                    );
                }

                let events = events
                    .find::<Evt>()
                    .flatten()
                    .filter(|event| {
                        event.service_id() == handler.service_id()
                            && event.job_id() == handler.job_id()
                    })
                    .collect::<Vec<_>>();

                if !events.is_empty() {
                    let _ = actionable_events.insert(idx, events);
                }
            }

            if !actionable_events.is_empty() {
                return Some(actionable_events);
            }
        }
    }

    async fn handle_event(&mut self, job_events: HashMap<usize, Vec<Evt>>) -> Result<(), Error> {
        use crate::tangle_subxt::tangle_testnet_runtime::api as TangleApi;
        const MAX_RETRY_COUNT: usize = 5;
        crate::info!("Handling actionable events ...");
        crate::info!(
            "Handling {} JobCalled Events @ block {}",
            job_events.len(),
            self.current_block.unwrap_or_default()
        );

        let mut tasks = Vec::new();
        for (handler_idx, calls) in job_events.iter() {
            let handler = &self.handlers[*handler_idx];
            let client = &self.client;
            for call in calls {
                let service_id = call.service_id();
                let call_id = call.id_of_call();
                let signer = handler.signer().clone();

                let task = async move {
                    let backoff = get_exponential_backoff::<MAX_RETRY_COUNT>();

                    Retry::spawn(backoff, || async {
                        let result = handler.handle(call).await?;
                        let response = TangleApi::tx()
                            .services()
                            .submit_result(service_id, call_id, result);
                        let _ = crate::tx::tangle::send(client, &signer, &response)
                            .await
                            .map_err(|err| Error::Client(err.to_string()))?;
                        Ok::<_, Error>(())
                    })
                    .await
                };

                tasks.push(task);
            }
        }

        let results = futures::future::join_all(tasks).await;
        // this event will be marked as handled if at least one handler succeeded.
        // this because, for the failed events, we already tried to handle them
        // many times (at this point), and there is no point in trying again.
        let mark_as_handled = results.iter().any(Result::is_ok);
        // also, for all the failed event handlers, we should print what went
        // wrong.
        for r in &results {
            if let Err(e) = r {
                crate::error!("Error from result: {e:?}");
            }
        }

        if mark_as_handled {
            crate::info!(
                "event handled successfully at block {}",
                self.current_block.unwrap_or_default()
            );
        } else {
            crate::error!("Error while handling event, all handlers failed.");
            crate::warn!("Restarting event watcher ...");
            // this a transient error, so we will retry again.
            return Ok(());
        }

        Ok(())
    }

    async fn execute(&mut self) -> Result<(), Error> {
        const MAX_RETRY_COUNT: usize = 10;

        for handler in &self.handlers {
            handler.init().await;
        }

        let mut backoff = get_exponential_backoff::<MAX_RETRY_COUNT>();

        let mut retry_count = 0;
        loop {
            match self.run_event_loop().await {
                Ok(_) => break Ok(()),
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

impl<Evt: Send + Sync + HasServiceAndJobId + 'static> SubstrateWatcherWrapper<Evt> {
    async fn run_event_loop(&mut self) -> Result<(), Error> {
        while let Some(events) = self.next_event().await {
            self.handle_event(events).await?;
        }

        crate::warn!("Event listener has stopped");
        Err(Error::Other("Event listener has stopped".to_string()))
    }
}
