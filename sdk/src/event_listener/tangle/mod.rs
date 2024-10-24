use crate::clients::tangle::runtime::{TangleClient, TangleConfig};
use crate::event_listener::markers::IsTangle;
use crate::event_listener::EventListener;
use crate::Error;
use async_trait::async_trait;
use gadget_blueprint_proc_macro_core::FieldType;
use sp_core::crypto::AccountId32;
use std::collections::VecDeque;
use std::marker::PhantomData;
use subxt::backend::StreamOfResults;
use subxt_core::events::EventDetails;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::call::{Job, ServiceId};
use tokio::sync::Mutex;

pub mod jobs;

pub struct TangleEventListener<Evt, Ctx> {
    current_block: Option<u32>,
    job_id: Job,
    service_id: ServiceId,
    listener: Mutex<StreamOfResults<subxt::blocks::Block<TangleConfig, TangleClient>>>,
    context: Ctx,
    signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
    client: TangleClient,
    enqueued_events: VecDeque<EventDetails<TangleConfig>>,
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
    pub evt: EventDetails<TangleConfig>,
    pub context: Ctx,
    pub block_number: BlockNumber,
    pub signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
    pub client: TangleClient,
    pub job_id: Job,
    pub service_id: ServiceId,
    _pd: PhantomData<Evt>,
}

impl<Evt, Ctx> IsTangle for TangleEventListener<Evt, Ctx> {}

#[async_trait]
impl<Evt: Send + 'static, Ctx: Clone + Send + Sync + 'static>
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
            if let Some(evt) = self.enqueued_events.pop_front() {
                return Some(TangleEvent {
                    evt,
                    context: self.context.clone(),
                    signer: self.signer.clone(),
                    block_number: self.current_block?,
                    client: self.client.clone(),
                    job_id: self.job_id,
                    service_id: self.service_id,
                    _pd: PhantomData,
                });
            }

            let next_events = self.listener.get_mut().next().await?.ok()?;
            let block_number = next_events.number();
            self.current_block = Some(block_number);

            let next_events = next_events.events().await.ok()?;
            let mut root_events = next_events.iter().flatten().collect::<VecDeque<_>>();

            crate::info!("Found {} possible events ...", root_events.len());

            if let Some(evt) = root_events.pop_front() {
                if !root_events.is_empty() {
                    // Store for the next iteration; we can override this since we know
                    // by this point in the code there are no more events to process in
                    // the queue
                    self.enqueued_events = root_events;
                }

                return Some(TangleEvent {
                    evt,
                    context: self.context.clone(),
                    signer: self.signer.clone(),
                    block_number,
                    client: self.client.clone(),
                    job_id: self.job_id,
                    service_id: self.service_id,
                    _pd: PhantomData,
                });
            }
        }
    }

    async fn handle_event(&mut self, _event: TangleEvent<Evt, Ctx>) -> Result<(), Error> {
        unimplemented!("placeholder; will be removed")
    }
}

pub trait FieldTypeToValue: Sized {
    fn to_value(&self, field_type: FieldType) -> Self;
}

macro_rules! impl_field_type_to_value {
    ($($t:ty => $f:pat),*) => {
        $(
            impl FieldTypeToValue for $t {
                fn to_value(&self, field_type: FieldType) -> Self {
                    match field_type {
                        $f => self.clone(),
                        _ => panic!("Invalid field type!"),
                    }
                }
            }
        )*
    };
}

impl_field_type_to_value!(
    u8 => FieldType::Uint8,
    u16 => FieldType::Uint16,
    u32 => FieldType::Uint32,
    u64 => FieldType::Uint64,
    i8 => FieldType::Int8,
    i16 => FieldType::Int16,
    i32 => FieldType::Int32,
    i64 => FieldType::Int64,
    u128 => FieldType::Uint128,
    i128 => FieldType::Int128,
    f64 => FieldType::Float64,
    bool => FieldType::Bool,
    AccountId32 => FieldType::AccountId
);

impl FieldTypeToValue for String {
    fn to_value(&self, field_type: FieldType) -> Self {
        match field_type {
            FieldType::String => self.clone(),
            _ => panic!("Invalid field type!"),
        }
    }
}
