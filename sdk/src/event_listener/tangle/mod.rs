use crate::clients::tangle::runtime::{TangleClient, TangleConfig};
use crate::event_listener::markers::IsTangle;
use crate::event_listener::EventListener;
use crate::Error;
use async_trait::async_trait;
use gadget_blueprint_proc_macro_core::FieldType;
pub use subxt_core::utils::AccountId32;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use subxt::backend::StreamOfResults;
use subxt_core::events::EventDetails;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::BoundedString;
pub use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field;
use tangle_subxt::tangle_testnet_runtime::api::services::calls::types::call::{Job, ServiceId};
use tangle_subxt::tangle_testnet_runtime::api::services::events::job_called;
use tangle_subxt::tangle_testnet_runtime::api::services::events::job_called::CallId;
use tokio::sync::Mutex;

pub mod jobs;

pub struct TangleEventListener<Ctx, Evt = ()> {
    current_block: Option<u32>,
    job_id: Job,
    service_id: ServiceId,
    listener: Mutex<StreamOfResults<subxt::blocks::Block<TangleConfig, TangleClient>>>,
    context: Ctx,
    signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
    client: TangleClient,
    has_stopped: Arc<AtomicBool>,
    stopper_tx: Arc<parking_lot::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    enqueued_events: VecDeque<EventDetails<TangleConfig>>,
    _phantom: PhantomData<Evt>,
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
#[derive(Clone)]
pub struct TangleEvent<Ctx, Evt = ()> {
    pub evt: EventDetails<TangleConfig>,
    pub context: Ctx,
    pub call_id: Option<CallId>,
    pub args: job_called::Args,
    pub block_number: BlockNumber,
    pub signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
    pub client: TangleClient,
    pub job_id: Job,
    pub service_id: ServiceId,
    pub stopper: Arc<parking_lot::Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    pub _phantom: PhantomData<Evt>,
}

impl<Ctx, Evt> TangleEvent<Ctx, Evt> {
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

impl<Ctx> IsTangle for TangleEventListener<Ctx> {}

#[async_trait]
impl<Ctx: Clone + Send + Sync + 'static, Evt: Send + Sync + 'static>
    EventListener<TangleEvent<Ctx, Evt>, TangleListenerInput<Ctx>>
    for TangleEventListener<Ctx, Evt>
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

        let (tx, rx) = tokio::sync::oneshot::channel();
        let has_stopped = Arc::new(AtomicBool::new(false));

        let has_stopped_clone = has_stopped.clone();

        let background_task = async move {
            let _ = rx.await;
            has_stopped_clone.store(false, Ordering::SeqCst);
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
            _phantom: PhantomData,
        })
    }

    async fn next_event(&mut self) -> Option<TangleEvent<Ctx, Evt>> {
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
                    _phantom: PhantomData,
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
                    call_id: None,
                    stopper: self.stopper_tx.clone(),
                    args: vec![],
                    block_number,
                    client: self.client.clone(),
                    job_id: self.job_id,
                    service_id: self.service_id,
                    _phantom: PhantomData,
                });
            }
        }
    }

    async fn handle_event(&mut self, _event: TangleEvent<Ctx, Evt>) -> Result<(), Error> {
        unimplemented!("placeholder; will be removed")
    }
}

pub trait FieldTypeIntoValue: Sized {
    fn convert(field: Field<AccountId32>, field_type: FieldType) -> Self;
}

pub trait ValueIntoFieldType {
    fn into_field_type(self) -> Field<AccountId32>;
}

macro_rules! impl_value_to_field_type {
    ($($t:ty => $j:path),*) => {
        $(
            impl ValueIntoFieldType for $t {
                fn into_field_type(self) -> Field<AccountId32> {
                    $j(self)
                }
            }
        )*
    };
}

impl_value_to_field_type!(
    u8 => Field::Uint8,
    u16 => Field::Uint16,
    u32 => Field::Uint32,
    u64 => Field::Uint64,
    i8 => Field::Int8,
    i16 => Field::Int16,
    i32 => Field::Int32,
    i64 => Field::Int64,
    bool => Field::Bool,
    AccountId32 => Field::AccountId
);

impl<T: ValueIntoFieldType> ValueIntoFieldType for Vec<T> {
    fn into_field_type(self) -> Field<AccountId32> {
        Field::Array(BoundedVec(
            self.into_iter()
                .map(ValueIntoFieldType::into_field_type)
                .collect(),
        ))
    }
}

impl<T: ValueIntoFieldType, const N: usize> ValueIntoFieldType for [T; N] {
    fn into_field_type(self) -> Field<AccountId32> {
        Field::Array(BoundedVec(
            self.into_iter()
                .map(ValueIntoFieldType::into_field_type)
                .collect(),
        ))
    }
}

impl<T: ValueIntoFieldType> ValueIntoFieldType for Option<T> {
    fn into_field_type(self) -> Field<AccountId32> {
        match self {
            Some(val) => val.into_field_type(),
            None => Field::None,
        }
    }
}

impl ValueIntoFieldType for String {
    fn into_field_type(self) -> Field<AccountId32> {
        Field::String(BoundedString(BoundedVec(self.into_bytes())))
    }
}

macro_rules! impl_field_type_to_value {
    ($($t:ty => $f:pat => $j:path),*) => {
        $(
            impl FieldTypeIntoValue for $t {
                fn convert(field: Field<AccountId32>, field_type: FieldType) -> Self {
                    match field_type {
                        $f => {
                            let $j (val) = field else {
                                panic!("Invalid field type!");
                            };

                            val
                        },
                        _ => panic!("Invalid field type!"),
                    }
                }
            }
        )*
    };
}

impl_field_type_to_value!(
    u16 => FieldType::Uint16 => Field::Uint16,
    u32 => FieldType::Uint32 => Field::Uint32,
    u64 => FieldType::Uint64 => Field::Uint64,
    i8 => FieldType::Int8 => Field::Int8,
    i16 => FieldType::Int16 => Field::Int16,
    i32 => FieldType::Int32 => Field::Int32,
    i64 => FieldType::Int64 => Field::Int64,
    bool => FieldType::Bool => Field::Bool,
    AccountId32 => FieldType::AccountId => Field::AccountId
);

impl FieldTypeIntoValue for String {
    fn convert(field: Field<AccountId32>, field_type: FieldType) -> Self {
        match field_type {
            FieldType::String => {
                let Field::String(val) = field else {
                    panic!("Invalid field type!");
                };

                String::from_utf8(val.0 .0).expect("Bad String from pallet Field")
            }
            _ => panic!("Invalid field type!"),
        }
    }
}

pub struct TangleResult<Res> {
    pub results: Res,
    pub service_id: ServiceId,
    pub call_id: job_called::CallId,
    pub client: TangleClient,
    pub signer: crate::keystore::TanglePairSigner<sp_core::sr25519::Pair>,
}
