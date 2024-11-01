use crate::event_listener::EventListener;
use crate::store::LocalDatabase;
use crate::{error, Error};
pub use alloy_contract::ContractInstance;
use alloy_contract::Event;
pub use alloy_network::Ethereum;
use alloy_provider::Provider;
pub use alloy_provider::RootProvider;
use alloy_rpc_types::{BlockNumberOrTag, Filter};
use alloy_sol_types::SolEvent;
pub use alloy_transport::BoxTransport;
use std::collections::VecDeque;
use std::time::Duration;
use uuid::Uuid;

pub type AlloyRootProvider = RootProvider<BoxTransport>;
pub type AlloyContractInstance = ContractInstance<BoxTransport, AlloyRootProvider, Ethereum>;

pub struct EvmContractEventListener<E: SolEvent + Send + 'static> {
    instance: AlloyContractInstance,
    chain_id: u64,
    local_db: LocalDatabase<u64>,
    should_cooldown: bool,
    enqueued_events: VecDeque<(E, alloy_rpc_types::Log)>,
}

#[async_trait::async_trait]
impl<E: SolEvent + Send + Sync + 'static>
    EventListener<(E, alloy_rpc_types::Log), AlloyContractInstance>
    for EvmContractEventListener<E>
{
    async fn new(context: &AlloyContractInstance) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let provider = context.provider().root();
        // Add more detailed error handling and logging
        let chain_id = provider
            .get_chain_id()
            .await
            .map_err(|err| Error::Client(format!("Failed to get chain ID: {}", err)))?;

        let local_db = LocalDatabase::open(format!("./db/{}", Uuid::new_v4()));
        Ok(Self {
            chain_id,
            should_cooldown: false,
            enqueued_events: VecDeque::new(),
            local_db,
            instance: context.clone(),
        })
    }

    async fn next_event(&mut self) -> Option<(E, alloy_rpc_types::Log)> {
        if let Some(event) = self.enqueued_events.pop_front() {
            return Some(event);
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

        // Query events
        let events_filter = Event::new(contract.provider(), Filter::new())
            .address(*contract.address())
            .from_block(BlockNumberOrTag::Number(block + 1))
            .to_block(BlockNumberOrTag::Number(dest_block))
            .event_signature(E::SIGNATURE_HASH);

        crate::info!("Querying events for filter, address: {}, from_block: {}, to_block: {}, event_signature: {}", contract.address(), block + 1, dest_block, E::SIGNATURE_HASH);
        match events_filter.query().await {
            Ok(events) => {
                let events = events.into_iter().collect::<VecDeque<_>>();
                self.local_db.set(
                    &format!("LAST_BLOCK_NUMBER_{}", contract.address()),
                    dest_block,
                );

                self.local_db.set(
                    &format!("TARGET_BLOCK_{}", contract.address()),
                    target_block_number,
                );

                if events.is_empty() {
                    self.should_cooldown = true;
                    return self.next_event().await;
                }

                self.enqueued_events = events;

                self.next_event().await
            }
            Err(e) => {
                error!(?e, %self.chain_id, "Error while querying events");
                None
            }
        }
    }
}
