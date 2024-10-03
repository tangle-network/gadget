use crate::clients::tangle::runtime::TangleConfig;
use crate::events_watcher::evm::{Config as ConfigT, EventWatcher};
use crate::events_watcher::substrate::{EventHandlerFor, SubstrateEventWatcher};
use crate::Error;
use alloy_network::Ethereum;
use async_trait::async_trait;
use backon::{ConstantBuilder, ExponentialBuilder, Retryable};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use std::collections::HashMap;
use std::default::Default;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use subxt::backend::StreamOfResults;
use subxt::OnlineClient;
use tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
use tokio::sync::Mutex;

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

/// A generic event listener for the Tangle network.
pub struct TangleEventListener;

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

struct EthereumWatcherWrapper<
    T: EventWatcher<C>,
    Ctx: Send + Sync + 'static,
    C: ConfigT<N = Ethereum>,
> {
    listener: T,
    _phantom: std::marker::PhantomData<(Ctx, C)>,
}

struct SubstrateWatcherWrapper<Ctx: Send + Sync + 'static> {
    handlers: Vec<EventHandlerFor<TangleConfig>>,
    client: OnlineClient<TangleConfig>,
    current_block: Option<u32>,
    listener:
        Mutex<StreamOfResults<subxt::blocks::Block<TangleConfig, OnlineClient<TangleConfig>>>>,
    _pd: std::marker::PhantomData<Ctx>,
}

impl<Config: ConfigT<N = Ethereum>, Ctx: Send + Sync + 'static, Watcher: EventWatcher<Config>>
    From<Watcher> for EthereumWatcherWrapper<Watcher, Ctx, Config>
{
    fn from(value: Watcher) -> Self {
        Self {
            listener: value,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<Config: ConfigT<N = Ethereum>, Ctx: Send + Sync + 'static, Watcher: EventWatcher<Config>>
    EventListener<Watcher::Event, Ctx> for EthereumWatcherWrapper<Watcher, Ctx, Config>
{
    async fn new(_context: &Ctx) -> Result<Self, Error>
    where
        Self: Sized,
    {
        unreachable!("For internal dev use only")
    }

    async fn next_event(&mut self) -> Option<Watcher::Event> {
        if let Err(err) = self.listener.run().await {
            crate::error!("Error running event watcher: {err}");
        }

        None
    }

    // No-op for now since logic already implemented inside Watcher::run
    async fn handle_event(&mut self, _event: Watcher::Event) -> Result<(), Error> {
        unreachable!("This function should not be called")
    }
}

/*
impl<
        Config: subxt::Config + Send + Sync + 'static,
        Ctx: Send + Sync + 'static,
        Watcher: SubstrateEventWatcher<Config>,
    > From<Watcher> for SubstrateWatcherWrapper<Watcher, Ctx, Config>
{
    fn from(value: Watcher) -> Self {
        Self {
            listener: value,
            _phantom: std::marker::PhantomData,
        }
    }
}*/

#[async_trait]
impl<Watcher: SubstrateEventWatcher<TangleConfig>>
    EventListener<HashMap<usize, Vec<JobCalled>>, Watcher> for SubstrateWatcherWrapper<Watcher>
{
    async fn new(ctx: &Watcher) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let listener = Mutex::new(ctx.client().blocks().subscribe_finalized().await?);
        Ok(Self {
            listener,
            client: ctx.client().clone(),
            current_block: None,
            handlers: ctx.handlers().iter().map(|r| r.clone()).collect(),
            _pd: std::marker::PhantomData,
        })
    }

    async fn next_event(&mut self) -> Option<HashMap<usize, Vec<JobCalled>>> {
        loop {
            let next_block = self.listener.get_mut().next().await?.ok()?;
            let block_id = next_block.number();
            self.current_block = Some(block_id);

            let events = next_block.events().await.ok()?;
            let mut actionable_events = HashMap::new();
            for (idx, handler) in self.handlers.iter().enumerate() {
                for evt in events.iter() {
                    if let Ok(_evt) = evt {
                        crate::info!(
                            "Event found || required: sid={}, jid={}",
                            handler.service_id(),
                            handler.job_id()
                        );
                    }
                }

                let events = events
                    .find::<JobCalled>()
                    .flatten()
                    .filter(|event| {
                        event.service_id == handler.service_id() && event.job == handler.job_id()
                    })
                    .collect::<Vec<_>>();

                if events.len() > 0 {
                    actionable_events.insert(idx, events);
                }
            }

            if actionable_events.len() > 0 {
                return Some(actionable_events);
            }
        }
    }

    async fn handle_event(
        &mut self,
        job_events: HashMap<usize, Vec<JobCalled>>,
    ) -> Result<(), Error> {
        use crate::tangle_subxt::tangle_testnet_runtime::api as TangleApi;
        const MAX_RETRY_COUNT: usize = 5;
        crate::info!("Handling actionable events ...");
        crate::info!(
            "Handling {} JobCalled Events @ block {}",
            job_events.len(),
            self.current_block.clone().unwrap_or_default()
        );

        let mut tasks = FuturesOrdered::new();
        for (handler_idx, calls) in job_events.iter() {
            let handler = &self.handlers[*handler_idx];
            let client = &self.client;
            for call in calls {
                let task = || {
                    Box::pin(async {
                        let signer = handler.signer();
                        let service_id = call.service_id;
                        let call_id = call.call_id;
                        let result = handler.handle(call).await?;
                        let response = TangleApi::tx()
                            .services()
                            .submit_result(service_id, call_id, result);
                        crate::tx::tangle::send(client, signer, &response)
                            .await
                            .map_err(|err| Error::Client(err.to_string()))?;
                        Ok::<_, Error>(())
                    }) as Pin<Box<dyn Send + Future<Output = _>>>
                };

                let backoff = ConstantBuilder::default()
                    .with_delay(Duration::from_millis(100))
                    .with_max_times(MAX_RETRY_COUNT);

                let task = task.retry(backoff);
                tasks.push_back(task)
            }
        }

        let results = tasks.collect::<Vec<_>>().await;
        // this event will be marked as handled if at least one handler succeeded.
        // this because, for the failed events, we arleady tried to handle them
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
                self.current_block.clone().unwrap_or_default()
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
        const MAX_RETRY_COUNT: usize = 5;

        for handler in &self.handlers {
            handler.init();
        }

        let backoff = ExponentialBuilder::default().with_max_times(usize::MAX);
        let meta_task = || async {
            while let Some(events) = self.next_event().await {
                self.handle_event(events).await?
            }

            Err::<(), Error>(Error::Other("Event listener has stopped".to_string()))
        };

        meta_task.retry(backoff).await?;
        Ok(())
    }
}

pub trait IntoEvmEventListener<
    Event: Send + Sync + 'static,
    Ctx: Send + Sync + 'static,
    Ext: Send + Sync + 'static = (),
>
{
    fn into_evm_event_listener(self) -> GenericEventListener<Event, Ctx, Ext>;
}

impl<Config: ConfigT<N = Ethereum>, Ctx: Send + Sync + 'static, Watcher: EventWatcher<Config>>
    IntoEvmEventListener<Watcher::Event, Ctx, (Config, Watcher)> for Watcher
{
    fn into_evm_event_listener(
        self,
    ) -> GenericEventListener<Watcher::Event, Ctx, (Config, Watcher)> {
        GenericEventListener::new(EthereumWatcherWrapper::from(self))
    }
}
