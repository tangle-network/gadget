use crate::clients::tangle::runtime::{TangleClient, TangleConfig};
use crate::event_listener::markers::IsTangle;
use crate::event_listener::EventListener;
use crate::Error;
use async_trait::async_trait;
pub use subxt_core::utils::AccountId32;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use subxt::backend::StreamOfResults;
use subxt_core::events::{EventDetails, StaticEvent};
pub use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::call::{Job, ServiceId};
use tangle_subxt::tangle_testnet_runtime::api::services::events::job_called;
use tangle_subxt::tangle_testnet_runtime::api::services::events::job_called::CallId;
use tokio::sync::Mutex;

pub mod jobs;

pub struct TangleEventListener<C, E: EventMatcher = AllEvents> {
    current_block: Option<u32>,
    job_id: Job,
    service_id: ServiceId,
    listener: Mutex<StreamOfResults<subxt::blocks::Block<TangleConfig, TangleClient>>>,
    context: C,
    signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
    client: TangleClient,
    has_stopped: Arc<AtomicBool>,
    stopper_tx: Arc<parking_lot::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    enqueued_events: VecDeque<E::Output>,
}

pub type BlockNumber = u32;

#[derive(Clone)]
pub struct TangleListenerInput<C> {
    pub client: TangleClient,
    pub job_id: Job,
    pub service_id: ServiceId,
    pub signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
    pub context: C,
}

/// Emitted by the [`TangleEventListener`] when a new event is received.
///
/// Root events are preferred to be used as the E, as then the application can
/// sort through a series of events to find the ones it is interested in for
/// pre-processing.
#[derive(Clone)]
pub struct TangleEvent<C, E: EventMatcher = AllEvents> {
    pub evt: E::Output,
    pub context: C,
    pub call_id: Option<CallId>,
    pub args: job_called::Args,
    pub block_number: BlockNumber,
    pub signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
    pub client: TangleClient,
    pub job_id: Job,
    pub service_id: ServiceId,
    pub stopper: Arc<parking_lot::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl<C, E: EventMatcher> TangleEvent<C, E> {
    /// Stops the event listener
    pub fn stop(&self) -> bool {
        let mut lock = self.stopper.lock();
        if let Some(tx) = lock.take() {
            tx.send(()).is_ok()
        } else {
            false
        }
    }
}

pub trait EventMatcher: Send + 'static {
    type Output: Send + 'static;
    fn try_decode(event: EventDetails<TangleConfig>) -> Option<Self::Output>;
}

impl<T: StaticEvent + Send + 'static> EventMatcher for T {
    type Output = T;
    fn try_decode(event: EventDetails<TangleConfig>) -> Option<Self::Output> {
        event.as_event::<T>().ok().flatten()
    }
}

#[derive(Copy, Clone)]
pub struct AllEvents;

impl EventMatcher for AllEvents {
    type Output = EventDetails<TangleConfig>;
    fn try_decode(event: EventDetails<TangleConfig>) -> Option<Self::Output> {
        Some(event)
    }
}

impl<C, E: EventMatcher> IsTangle for TangleEventListener<C, E> {}

pub trait ThreadSafeCloneable: Clone + Send + Sync + 'static {}
impl<T: Clone + Send + Sync + 'static> ThreadSafeCloneable for T {}

#[async_trait]
impl<C: ThreadSafeCloneable, E: EventMatcher>
    EventListener<TangleEvent<C, E>, TangleListenerInput<C>> for TangleEventListener<C, E>
{
    async fn new(context: &TangleListenerInput<C>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let TangleListenerInput {
            client,
            job_id,
            service_id,
            context,
            signer,
        } = context;

        let listener = Mutex::new(client.blocks().subscribe_finalized().await?);

        let (tx, rx) = tokio::sync::oneshot::channel();
        let has_stopped = Arc::new(AtomicBool::new(false));

        let has_stopped_clone = has_stopped.clone();

        let background_task = async move {
            let _ = rx.await;
            has_stopped_clone.store(true, Ordering::SeqCst);
        };

        drop(tokio::task::spawn(background_task));

        Ok(Self {
            listener,
            current_block: None,
            job_id: *job_id,
            service_id: *service_id,
            context: context.clone(),
            client: client.clone(),
            signer: signer.clone(),
            stopper_tx: Arc::new(parking_lot::Mutex::new(Some(tx))),
            has_stopped,
            enqueued_events: VecDeque::new(),
        })
    }

    async fn next_event(&mut self) -> Option<TangleEvent<C, E>> {
        loop {
            if self.has_stopped.load(Ordering::SeqCst) {
                return None;
            }

            if let Some(evt) = self.enqueued_events.pop_front() {
                return Some(TangleEvent {
                    evt,
                    context: self.context.clone(),
                    signer: self.signer.clone(),
                    call_id: None,
                    stopper: self.stopper_tx.clone(),
                    args: vec![],
                    block_number: self.current_block?,
                    client: self.client.clone(),
                    job_id: self.job_id,
                    service_id: self.service_id,
                });
            }

            let next_events = self.listener.get_mut().next().await?.ok()?;
            let block_number = next_events.number();
            self.current_block = Some(block_number);

            let events = next_events
                .events()
                .await
                .ok()?
                .iter()
                .filter_map(|r| r.ok().and_then(E::try_decode))
                .collect::<VecDeque<_>>();

            crate::debug!("Found {} possible events ...", events.len());
            self.enqueued_events = events;
        }
    }
}

pub struct TangleResult<R: serde::Serialize> {
    pub results: R,
    pub service_id: ServiceId,
    pub call_id: job_called::CallId,
    pub client: TangleClient,
    pub signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
}
