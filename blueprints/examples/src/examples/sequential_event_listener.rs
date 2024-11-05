use alloy_sol_types::SolEvent;

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
                const EXPIRY_BLOCKS: u64 = 1000;
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::U256;
    use alloy_sol_types::{sol, SolEvent}; 
    use alloy_provider::{Provider, ProviderBuilder};
    use alloy_signer_local::PrivateKeySigner;
    use alloy_contract::ContractInstance;
    use std::str::FromStr;

    // Define Counter events
    sol! {
        event Number1Incremented(bytes32 indexed id, uint256 indexed oldValue, uint256 indexed newValue);
        event Number2Incremented(bytes32 indexed id, uint256 indexed oldValue, uint256 indexed newValue);
        event Number3Incremented(bytes32 indexed id, uint256 indexed oldValue, uint256 indexed newValue);
    }

    sequential_event_listener!(CounterEventListener, Number1Incremented, Number2Incremented, Number3Incremented);

    struct CounterEventCorrelator;
    impl EventCorrelation<Number1Incremented, Number2Incremented> for CounterEventCorrelator {
        fn are_correlated(&self, n1: &Number1Incremented, n2: &Number2Incremented) -> bool {
            // Correlate if they have the same identifier
            n1.id == n2.id
        }
    }

    impl EventCorrelation<Number2Incremented, Number3Incremented> for CounterEventCorrelator {
        fn are_correlated(&self, n2: &Number2Incremented, n3: &Number3Incremented) -> bool {
            // Correlate if they have the same identifier
            n2.id == n3.id
        }
    }

    const CONTRACT_SOURCE: &str = r#"
        // SPDX-License-Identifier: MIT
        pragma solidity ^0.8.0;

        contract ComplexCounter {
            struct CounterSet {
                uint256 number1;
                uint256 number2;
                uint256 number3;
            }

            mapping(bytes32 => CounterSet) public counters;

            event Number1Incremented(bytes32 indexed id, uint256 indexed oldValue, uint256 indexed newValue);
            event Number2Incremented(bytes32 indexed id, uint256 indexed oldValue, uint256 indexed newValue);
            event Number3Incremented(bytes32 indexed id, uint256 indexed oldValue, uint256 indexed newValue);

            function incrementNumber1(bytes32 id) public {
                uint256 oldValue = counters[id].number1;
                counters[id].number1++;
                emit Number1Incremented(id, oldValue, counters[id].number1);
            }

            function incrementNumber2(bytes32 id) public {
                uint256 oldValue = counters[id].number2;
                counters[id].number2++;
                emit Number2Incremented(id, oldValue, counters[id].number2);
            }

            function incrementNumber3(bytes32 id) public {
                uint256 oldValue = counters[id].number3;
                counters[id].number3++;
                emit Number3Incremented(id, oldValue, counters[id].number3);
            }

            function getCounterSet(bytes32 id) public view returns (uint256, uint256, uint256) {
                CounterSet memory set = counters[id];
                return (set.number1, set.number2, set.number3);
            }
        }
    "#;

    #[tokio::test]
    async fn test_complex_counter_sequence() {
        // Start anvil node
        let provider = ProviderBuilder::new()
            .on_http("http://localhost:8545")
            .boxed()
            .root();

        // Get test account
        let owner = PrivateKeySigner::from_bytes(&[1; 32].into()).unwrap();

        // Deploy contract
        let contract = Contract::new(
            CONTRACT_SOURCE.to_string(),
            "ComplexCounter".to_string(), 
            provider.clone(),
        )
        .unwrap();

        let contract_addr = contract.deploy().await.unwrap();
        let contract = ContractInstance::new(contract_addr, contract.abi().clone(), provider.clone());

        // Create event listener
        let mut listener = CounterEventListener::new();
        listener.add_correlator(Box::new(CounterEventCorrelator));

        // Test sequence of events for multiple IDs
        let id1 = [1u8; 32];
        let id2 = [2u8; 32];

        // Increment numbers for id1
        contract
            .call("incrementNumber1", (id1,))
            .sign_with(owner.clone())
            .send()
            .await
            .unwrap();

        contract
            .call("incrementNumber2", (id1,))
            .sign_with(owner.clone())
            .send()
            .await
            .unwrap();

        contract
            .call("incrementNumber3", (id1,))
            .sign_with(owner.clone())
            .send()
            .await
            .unwrap();

        // Increment some numbers for id2 (but not all)
        contract
            .call("incrementNumber1", (id2,))
            .sign_with(owner.clone())
            .send()
            .await
            .unwrap();

        contract
            .call("incrementNumber2", (id2,))
            .sign_with(owner.clone())
            .send()
            .await
            .unwrap();

        // Get logs and process events
        let logs = contract.get_logs(0..=provider.get_block_number().await.unwrap()).await.unwrap();
        
        for log in logs {
            let topics = log.topics();
            let data = log.data();
            
            if let Ok(event) = Number1Incremented::decode_log(topics, data) {
                listener.process_event(event, log.clone());
            } else if let Ok(event) = Number2Incremented::decode_log(topics, data) {
                listener.process_event(event, log.clone());
            } else if let Ok(event) = Number3Incremented::decode_log(topics, data) {
                listener.process_event(event, log.clone());
            }
        }

        // Verify sequences were detected
        // Should have one complete sequence for id1 but not for id2
        assert_eq!(listener.completed_sequences.len(), 1);
        
        // Verify counter values
        let (n1, n2, n3) = contract.call("getCounterSet", (id1,)).call().await.unwrap();
        assert_eq!(n1, U256::from(1));
        assert_eq!(n2, U256::from(1));
        assert_eq!(n3, U256::from(1));

        let (n1, n2, n3) = contract.call("getCounterSet", (id2,)).call().await.unwrap();
        assert_eq!(n1, U256::from(1));
        assert_eq!(n2, U256::from(1));
        assert_eq!(n3, U256::from(0));
    }
}
