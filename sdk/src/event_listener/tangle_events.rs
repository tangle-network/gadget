use crate::clients::tangle::runtime::{TangleClient, TangleConfig};
use crate::events_watcher::substrate::EventHandlerFor;
use crate::Error;
use async_trait::async_trait;
use std::collections::HashMap;
use subxt::backend::StreamOfResults;
use subxt::OnlineClient;
use subxt_core::events::StaticEvent;
use tangle_subxt::tangle_testnet_runtime::api::services::events::{
    job_called, JobCalled, JobResultSubmitted,
};
use tokio::sync::Mutex;
use tokio_retry::Retry;

use crate::event_listener::get_exponential_backoff;
use crate::event_listener::EventListener;
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
        let (client, handler) = ctx;
        let listener = Mutex::new(client.blocks().subscribe_finalized().await?);
        Ok(Self {
            listener,
            client: client.clone(),
            current_block: None,
            handlers: vec![handler.clone()],
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
