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
    listener: Mutex<StreamOfResults<subxt::blocks::Block<TangleConfig, TangleClient>>>,
    context: Ctx,
    signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
    client: TangleClient,
    enqueued_events: VecDeque<Evt>,
    _pd: PhantomData<Evt>,
}

pub type BlockNumber = u32;

#[derive(Clone)]
pub struct TangleListenerInput<Ctx> {
    pub client: TangleClient,
    pub job_id: Job,
    pub service_id: ServiceId,
    pub signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
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
    pub signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
    pub client: TangleClient,
    pub job_id: Job,
    pub service_id: ServiceId,
}

#[async_trait]
impl<Evt: DecodeAsType + Send + 'static, Ctx: Clone + Send + Sync + 'static>
    EventListener<TangleEvent<Evt, Ctx>, TangleListenerInput<Ctx>>
    for TangleEventListener<Evt, Ctx>
{
    async fn new(context: &TangleListenerInput<Ctx>) -> Result<Self, Error>
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
        Ok(Self {
            listener,
            current_block: None,
            job_id: *job_id,
            service_id: *service_id,
            context: context.clone(),
            client: client.clone(),
            signer: signer.clone(),
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
                    signer: self.signer.clone(),
                    block_number: self.current_block?,
                    client: self.client.clone(),
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
                    signer: self.signer.clone(),
                    block_number,
                    client: self.client.clone(),
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
    pub args: Vec<Field<AccountId32>>,
    pub context: Ctx,
    pub client: TangleClient,
    pub signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
    pub block_number: BlockNumber,
    pub call_id: job_called::CallId,
    pub job_id: Job,
    pub service_id: ServiceId,
}

impl<Ctx> TangleJobEvent<Ctx> {
    pub fn result(self, result: Field<AccountId32>) -> Result<TangleJobResult, Error> {
        self.results(vec![result])
    }

    pub fn results<T: Into<types::submit_result::Result>>(
        self,
        results: T,
    ) -> Result<TangleJobResult, Error> {
        Ok(TangleJobResult {
            results: results.into(),
            service_id: self.service_id,
            call_id: self.call_id,
            client: self.client,
            signer: self.signer,
        })
    }
}

pub struct TangleJobResult {
    pub results: types::submit_result::Result,
    pub service_id: ServiceId,
    pub call_id: job_called::CallId,
    pub client: TangleClient,
    pub signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
}

pub async fn services_pre_processor<Ctx>(
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
                args,
                context: event.context,
                block_number: event.block_number,
                signer: event.signer,
                client: event.client,
                call_id,
                job_id: this_job_id,
                service_id: this_service_id,
            });
        }
    }

    Err(Error::SkipPreProcessedType)
}

/// By default, the tangle post-processor takes in a job result and submits the result on-chain
pub async fn services_post_processor(
    TangleJobResult {
        results,
        service_id,
        call_id,
        client,
        signer,
    }: TangleJobResult,
) -> Result<(), Error> {
    let response = TangleApi::tx()
        .services()
        .submit_result(service_id, call_id, results);
    let _ = crate::tx::tangle::send(&client, &signer, &response)
        .await
        .map_err(|err| Error::Client(err.to_string()))?;
    crate::info!("Submitted result on-chain");
    Ok(())
}

pub trait FieldTypeIntoConcrete<Concrete> {
    fn into_concrete(self) -> Concrete;
}

macro_rules! impl_field_type_into_concrete {
    ($concrete:ty, $variant:ident) => {
        impl FieldTypeIntoConcrete<$concrete> for Field<AccountId32> {
            fn into_concrete(self) -> $concrete {
                if let Field::$variant(val) = self {
                    val
                } else {
                    panic!("Expected $variant, got {:?}", self);
                }
            }
        }
    };
}

impl_field_type_into_concrete!(u8, Uint8);
impl_field_type_into_concrete!(u16, Uint16);
impl_field_type_into_concrete!(u32, Uint32);
impl_field_type_into_concrete!(u64, Uint64);
impl_field_type_into_concrete!(i8, Int8);
impl_field_type_into_concrete!(i16, Int16);
impl_field_type_into_concrete!(i32, Int32);
impl_field_type_into_concrete!(i64, Int64);
impl_field_type_into_concrete!(AccountId32, AccountId);

impl FieldTypeIntoConcrete<String> for Field<AccountId32> {
    fn into_concrete(self) -> String {
        if let Field::String(val) = self {
            String::from_utf8(val.0 .0).unwrap()
        } else {
            panic!("Expected $variant, got {:?}", self);
        }
    }
}

impl FieldTypeIntoConcrete<Vec<u8>> for Field<AccountId32> {
    fn into_concrete(self) -> Vec<u8> {
        if let Field::Bytes(val) = self {
            val.0
        } else {
            panic!("Expected $variant, got {:?}", self);
        }
    }
}
