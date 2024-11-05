use crate::event_listener::EventListener;
use crate::store::LocalDatabase;
use crate::{error, Error};
use alloy_contract::ContractInstance;
use alloy_contract::Event;
use alloy_provider::Provider;
use alloy_rpc_types::{BlockNumberOrTag, Filter, Log};
use alloy_sol_types::SolEvent;
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::time::Duration;
use uuid::Uuid;

const EXPIRY_BLOCKS: u64 = 1000;

/// Trait for correlating sequential events
pub trait EventCorrelation<E1: SolEvent, E2: SolEvent> {
    /// Returns true if the two events are correlated in sequence
    fn are_correlated(&self, first: &E1, second: &E2) -> bool;
}

/// Macro to create a sequential event listener with N events
#[macro_export]
macro_rules! sequential_event_listener {
    ($name:ident, $($event:ident),+) => {
        pub struct $name<$($event),+>
        where
            $($event: SolEvent + Send + Sync + 'static),+
        {
            instance: AlloyContractInstance,
            chain_id: u64,
            local_db: LocalDatabase<u64>,
            should_cooldown: bool,
            pending_sequences: HashMap<String, EventSequence<$($event),+>>,
            completed_sequences: VecDeque<($(($event, Log)),+)>,
            correlators: Vec<Box<dyn EventCorrelation<$event, $event> + Send + Sync>>,
            _phantom: PhantomData<($($event),+)>,
        }

        struct EventSequence<$($event),+> {
            $(
                $event: Option<($event, Log)>,
            )+
            sequence_id: String,
            last_update: u64,
        }

        impl<$($event),+> EventSequence<$($event),+> {
            fn new(sequence_id: String) -> Self {
                Self {
                    $(
                        $event: None,
                    )+
                    sequence_id,
                    last_update: 0,
                }
            }

            fn is_complete(&self) -> bool {
                $(self.$event.is_some())&&+
            }
        }

        #[async_trait::async_trait]
        impl<$($event),+> EventListener<($(($event, Log)),+), AlloyContractInstance>
            for $name<$($event),+>
        where
            $($event: SolEvent + Send + Sync + 'static),+
        {
            async fn new(context: &AlloyContractInstance) -> Result<Self, Error>
            where
                Self: Sized,
            {
                let provider = context.provider().root();
                let chain_id = provider
                    .get_chain_id()
                    .await
                    .map_err(|err| Error::Client(format!("Failed to get chain ID: {}", err)))?;

                let local_db = LocalDatabase::open(format!("./db/{}", Uuid::new_v4()));

                Ok(Self {
                    instance: context.clone(),
                    chain_id,
                    local_db,
                    should_cooldown: false,
                    pending_sequences: HashMap::new(),
                    completed_sequences: VecDeque::new(),
                    correlators: Vec::new(),
                    _phantom: PhantomData,
                })
            }

            async fn next_event(&mut self) -> Option<($(($event, Log)),+)> {
                if let Some(sequence) = self.completed_sequences.pop_front() {
                    return Some(sequence);
                }

                if self.should_cooldown {
                    tokio::time::sleep(Duration::from_millis(5000)).await;
                    self.should_cooldown = false;
                }

                let contract = &self.instance;
                let step = 100;
                let target_block_number: u64 = contract
                    .provider()
                    .get_block_number()
                    .await
                    .unwrap_or_default();

                let block = self
                    .local_db
                    .get(&format!("LAST_BLOCK_NUMBER_{}", contract.address()))
                    .unwrap_or(0);

                let should_cooldown = block >= target_block_number;
                if should_cooldown {
                    self.should_cooldown = true;
                    return self.next_event().await;
                }

                let dest_block = core::cmp::min(block + step, target_block_number);

                // Query all event types
                let mut all_events = Vec::new();
                $(
                    all_events.extend(self.query_events::<$event>(block + 1, dest_block).await);
                )+

                // Sort events by block number and index
                all_events.sort_by(|a, b| {
                    let block_cmp = a.block_number.cmp(&b.block_number);
                    if block_cmp == std::cmp::Ordering::Equal {
                        a.transaction_index.cmp(&b.transaction_index)
                    } else {
                        block_cmp
                    }
                });

                // Process events and update sequences
                for event in all_events {
                    self.process_event(event);
                }

                // Update block tracking
                self.local_db.set(
                    &format!("LAST_BLOCK_NUMBER_{}", contract.address()),
                    dest_block,
                );

                self.local_db.set(
                    &format!("TARGET_BLOCK_{}", contract.address()),
                    target_block_number,
                );

                // Clean up expired sequences
                self.cleanup_expired_sequences(target_block_number);

                self.completed_sequences.pop_front()
            }
        }

        impl<$($event),+> $name<$($event),+>
        where
            $($event: SolEvent + Send + Sync + 'static),+
        {
            pub fn with_correlators(mut self, correlators: Vec<Box<dyn EventCorrelation<$event, $event> + Send + Sync>>) -> Self {
                self.correlators = correlators;
                self
            }

            async fn query_events<E: SolEvent>(&self, from_block: u64, to_block: u64) -> Vec<Log> {
                let events_filter = Event::new(self.instance.provider(), Filter::new())
                    .address(*self.instance.address())
                    .from_block(BlockNumberOrTag::Number(from_block))
                    .to_block(BlockNumberOrTag::Number(to_block))
                    .event_signature(E::SIGNATURE_HASH);

                match events_filter.query().await {
                    Ok(events) => events,
                    Err(e) => {
                        error!(?e, %self.chain_id, "Error querying events");
                        Vec::new()
                    }
                }
            }

            fn process_event(&mut self, log: Log) {
                // Try to decode the event into each possible type
                $(
                    if let Ok(event) = $event::decode_log(&log) {
                        // Check existing sequences for correlation
                        for sequence in self.pending_sequences.values_mut() {
                            if self.events_correlate(&sequence, &event) {
                                sequence.$event = Some((event.clone(), log.clone()));
                                sequence.last_update = log.block_number.unwrap_or_default();

                                if sequence.is_complete() {
                                    if let Some(seq) = self.pending_sequences.remove(&sequence.sequence_id) {
                                        let completed = (
                                            $(
                                                seq.$event.unwrap(),
                                            )+
                                        );
                                        self.completed_sequences.push_back(completed);
                                    }
                                }
                                return;
                            }
                        }

                        // If no correlation found, create new sequence
                        let sequence_id = Uuid::new_v4().to_string();
                        let mut new_sequence = EventSequence::new(sequence_id.clone());
                        new_sequence.$event = Some((event, log.clone()));
                        new_sequence.last_update = log.block_number.unwrap_or_default();
                        self.pending_sequences.insert(sequence_id, new_sequence);
                    }
                )+
            }

            fn events_correlate<E1: SolEvent, E2: SolEvent>(&self, sequence: &EventSequence<$($event),+>, new_event: &E2) -> bool {
                for correlator in &self.correlators {
                    // Check correlation between consecutive events in the sequence
                    if let Some((prev_event, _)) = &sequence.E1 {
                        if correlator.are_correlated(prev_event, new_event) {
                            return true;
                        }
                    }
                }
                false
            }

            fn cleanup_expired_sequences(&mut self, current_block: u64) {
                self.pending_sequences.retain(|_, seq| {
                    current_block - seq.last_update < EXPIRY_BLOCKS
                });
            }
        }
    };
}

// Example usage:
// sequential_event_listener!(ThreeEventListener, Event1, Event2, Event3);
// sequential_event_listener!(TwoEventListener, Event1, Event2);
// sequential_event_listener!(FourEventListener, Event1, Event2, Event3, Event4);
