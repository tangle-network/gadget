use crate::clients::tangle::runtime::{TangleClient, TangleConfig};
use crate::event_listener::EventListener;
use crate::tangle_subxt::tangle_testnet_runtime::api::runtime_types::pallet_services::module::Event;
use crate::Error;
use async_trait::async_trait;
use std::collections::VecDeque;
use std::marker::PhantomData;
use subxt::backend::StreamOfResults;
use subxt::ext::scale_decode::DecodeAsType;
use subxt_core::utils::AccountId32;
use tangle_subxt::tangle_testnet_runtime::api as TangleApi;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::call::{Job, ServiceId};
use tangle_subxt::tangle_testnet_runtime::api::services::events::job_called;
use tokio::sync::Mutex;

pub struct TangleEventListener<Evt, Ctx> {
    current_block: Option<u32>,
    job_id: Job,
    service_id: ServiceId,
    listener:
        Mutex<StreamOfResults<subxt::blocks::Block<TangleConfig, TangleClient>>>,
    context: Ctx,
    enqueued_events: VecDeque<Evt>,
    _pd: PhantomData<Evt>,
}

pub type BlockNumber = u32;

#[derive(Clone)]
pub struct TangleSpecificContext<Ctx> {
    pub client: TangleClient,
    pub job_id: Job,
    pub service_id: ServiceId,
    pub context: Ctx,
}

/// Emitted by the [`TangleEventListener`] when a new event is received.
///
/// Root events are preferred to be used as the Evt, as then the application can
/// sort through a series of events to find the ones it is interested in for
/// pre-processing.
pub struct TangleEvent<Evt, Ctx> {
    pub root_event: Evt,
    pub context: Ctx,
    pub block_number: BlockNumber,
    pub job_id: Job,
    pub service_id: ServiceId,
}

#[async_trait]
impl<Evt: DecodeAsType + Send + 'static, Ctx: Clone + Send + Sync + 'static>
    EventListener<TangleEvent<Evt, Ctx>, TangleSpecificContext<Ctx>>
    for TangleEventListener<Evt, Ctx>
{
    async fn new(context: &TangleSpecificContext<Ctx>) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let TangleSpecificContext {
            client,
            job_id,
            service_id,
            context,
        } = context;

        let listener = Mutex::new(client.blocks().subscribe_finalized().await?);
        Ok(Self {
            listener,
            current_block: None,
            job_id: *job_id,
            service_id: *service_id,
            context: context.clone(),
            enqueued_events: VecDeque::new(),
            _pd: PhantomData,
        })
    }

    async fn next_event(&mut self) -> Option<TangleEvent<Evt, Ctx>> {
        loop {
            if let Some(enqueued_event) = self.enqueued_events.pop_front() {
                return Some(TangleEvent {
                    root_event: enqueued_event,
                    context: self.context.clone(),
                    block_number: self.current_block?,
                    job_id: self.job_id,
                    service_id: self.service_id,
                });
            }

            let next_events = self.listener.get_mut().next().await?.ok()?;
            let block_number = next_events.number();
            self.current_block = Some(block_number);

            let next_events = next_events.events().await.ok()?;
            let mut root_events = next_events
                .iter()
                .filter_map(|r| r.ok())
                .filter_map(|r| r.as_root_event::<Evt>().ok())
                .collect::<VecDeque<_>>();

            if let Some(root_event) = root_events.pop_front() {
                if !root_events.is_empty() {
                    // Store for the next iteration
                    self.enqueued_events = root_events;
                }

                return Some(TangleEvent {
                    root_event,
                    context: self.context.clone(),
                    block_number,
                    job_id: self.job_id,
                    service_id: self.service_id,
                });
            }
        }
    }

    async fn handle_event(&mut self, _event: TangleEvent<Evt, Ctx>) -> Result<(), Error> {
        unimplemented!("placeholder; will be removed")
    }
}

pub struct TangleJobEvent<Ctx> {
    pub result: Vec<Field<AccountId32>>,
    pub context: Ctx,
    pub block_number: BlockNumber,
    pub call_id: job_called::CallId,
    pub job_id: Job,
    pub service_id: ServiceId,
}

pub async fn tangle_default_pre_processor<Ctx>(
    event: TangleEvent<Event, Ctx>,
) -> Result<TangleJobEvent<Ctx>, Error> {
    let this_service_id = event.service_id;
    let this_job_id = event.job_id;

    if let Event::JobCalled {
        service_id,
        job,
        call_id,
        args,
        ..
    } = event.root_event
    {
        if job == this_job_id && service_id == this_service_id {
            return Ok(TangleJobEvent {
                result: args,
                context: event.context,
                block_number: event.block_number,
                call_id,
                job_id: this_job_id,
                service_id: this_service_id,
            });
        }
    }

    Err(Error::SkipPreProcessedType)
}

/// By default, the tangle post-processor takes in a job result and submits the result on-chain
async fn tangle_default_post_processor(
    result: types::submit_result::Result,
    service_id: ServiceId,
    call_id: job_called::CallId,
    client: TangleClient,
    signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
) -> Result<(), Error> {
    let response = TangleApi::tx()
        .services()
        .submit_result(service_id, call_id, result);
    let _ = crate::tx::tangle::send(&client, &signer, &response)
        .await
        .map_err(|err| Error::Client(err.to_string()))?;
    Ok(())
}
