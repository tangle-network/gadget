use crate::clients::tangle::runtime::{TangleClient, TangleConfig};
use crate::events_watcher::evm::{Config as ConfigT, EventWatcher};
use crate::events_watcher::substrate::EventHandlerFor;
use crate::Error;
use alloy_network::Ethereum;
use async_trait::async_trait;
use std::collections::HashMap;
use subxt::backend::StreamOfResults;
use subxt::OnlineClient;
use tangle_subxt::tangle_testnet_runtime::api::services::events::JobCalled;
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

/// A generic event listener for the Tangle network.
#[derive(Copy, Clone)]
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

pub struct SubstrateWatcherWrapper {
    handlers: Vec<EventHandlerFor<TangleConfig>>,
    client: OnlineClient<TangleConfig>,
    current_block: Option<u32>,
    listener:
        Mutex<StreamOfResults<subxt::blocks::Block<TangleConfig, OnlineClient<TangleConfig>>>>,
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

pub type SubstrateWatcherWrapperContext = (TangleClient, EventHandlerFor<TangleConfig>);
#[async_trait]
impl EventListener<HashMap<usize, Vec<JobCalled>>, SubstrateWatcherWrapperContext>
    for SubstrateWatcherWrapper
{
    async fn new(ctx: &SubstrateWatcherWrapperContext) -> Result<Self, Error>
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

    async fn next_event(&mut self) -> Option<HashMap<usize, Vec<JobCalled>>> {
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
                    .find::<JobCalled>()
                    .flatten()
                    .filter(|event| {
                        event.service_id == handler.service_id() && event.job == handler.job_id()
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
            self.current_block.unwrap_or_default()
        );

        let mut tasks = Vec::new();
        for (handler_idx, calls) in job_events.iter() {
            let handler = &self.handlers[*handler_idx];
            let client = &self.client;
            for call in calls {
                let service_id = call.service_id;
                let call_id = call.call_id;
                let signer = handler.signer().clone();
                let call = call.clone();

                let task = async move {
                    let backoff = ExponentialBackoff::from_millis(1000)
                        .factor(2)
                        .take(MAX_RETRY_COUNT);

                    Retry::spawn(backoff, || async {
                        let result = handler.handle(&call).await?;
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
        const MAX_RETRY_COUNT: usize = 5;

        for handler in &self.handlers {
            handler.init().await;
        }

        let mut backoff = ExponentialBackoff::from_millis(1000)
            .factor(2)
            .take(MAX_RETRY_COUNT);

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

impl SubstrateWatcherWrapper {
    async fn run_event_loop(&mut self) -> Result<(), Error> {
        while let Some(events) = self.next_event().await {
            self.handle_event(events).await?;
        }

        crate::warn!("Event listener has stopped");
        Err(Error::Other("Event listener has stopped".to_string()))
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
