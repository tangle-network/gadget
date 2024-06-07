use std::str::FromStr;

use alloy_primitives::Address;
use alloy_provider::{network::Ethereum, Provider};
use alloy_pubsub::Subscription;
use alloy_rpc_types::{Filter, Log};
use alloy_transport::Transport;
use async_trait::async_trait;
use eigen_utils::types::AvsError;

use super::{AvsManagersBindings, IncredibleSquaringTaskManager, SetupConfig};

#[async_trait]
pub trait AvsSubscriberTrait: Send + Sync {
    async fn subscribe_to_new_tasks(
        &self,
        new_task_created_chan: tokio::sync::mpsc::Sender<
            Log<IncredibleSquaringTaskManager::NewTaskCreated>,
        >,
    ) -> Result<Subscription<Log>, AvsError>;

    async fn subscribe_to_task_responses(
        &self,
        task_response_logs: tokio::sync::mpsc::Sender<
            Log<IncredibleSquaringTaskManager::TaskResponded>,
        >,
    ) -> Result<Subscription<Log>, AvsError>;
}

pub struct AvsSubscriber<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    avs_contract_bindings: AvsManagersBindings<T, P>,
    eth_client: P,
}

impl<T, P> AvsSubscriber<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub async fn from_config(config: &SetupConfig<T, P>) -> Result<Self, AvsError> {
        Self::new(
            config.registry_coordinator_addr,
            config.operator_state_retriever_addr,
            config.eth_client.clone(),
        )
        .await
    }

    pub async fn new(
        registry_coordinator_addr: Address,
        operator_state_retriever_addr: Address,
        eth_client: P,
    ) -> Result<Self, AvsError> {
        let avs_contract_bindings = AvsManagersBindings::new(
            registry_coordinator_addr,
            operator_state_retriever_addr,
            eth_client.clone(),
        )
        .await?;

        Ok(Self {
            avs_contract_bindings,
            eth_client,
        })
    }

    pub async fn build(
        registry_coordinator_addr: &str,
        operator_state_retriever_addr: &str,
        eth_client: P,
    ) -> Result<Self, AvsError> {
        Self::new(
            Address::from_str(registry_coordinator_addr).unwrap_or_default(),
            Address::from_str(operator_state_retriever_addr).unwrap_or_default(),
            eth_client,
        )
        .await
    }
}

#[async_trait]
impl<T, P> AvsSubscriberTrait for AvsSubscriber<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    async fn subscribe_to_new_tasks(
        &self,
        new_task_created_chan: tokio::sync::mpsc::Sender<
            Log<IncredibleSquaringTaskManager::NewTaskCreated>,
        >,
    ) -> Result<Subscription<Log>, AvsError> {
        let filter = Filter::new()
            .address(*self.avs_contract_bindings.task_manager.address())
            .event("NewTaskCreated");
        let subscription = self.eth_client.subscribe_logs(&filter).await?;

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(log) = subscription.recv() => {
                        match log.log_decode::<IncredibleSquaringTaskManager::NewTaskCreated>() {
                            Ok(event) => {
                                if new_task_created_chan.send(event).await.is_err() {
                                    log::error!("Failed to send new task created event");
                                }
                            }
                            Err(e) => log::error!("Failed to parse new task created event: {:?}", e),
                        };
                    }
                }
            }
        });

        Ok(subscription)
    }

    async fn subscribe_to_task_responses(
        &self,
        task_response_logs: tokio::sync::mpsc::Sender<
            Log<IncredibleSquaringTaskManager::TaskResponded>,
        >,
    ) -> Result<Subscription<Log>, AvsError> {
        let filter = Filter::new()
            .address(*self.avs_contract_bindings.task_manager.address())
            .event("TaskResponded");

        let subscription = self.eth_client.subscribe_logs(&filter).await?;
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Ok(log) = subscription.recv() => {
                        match log.log_decode::<IncredibleSquaringTaskManager::TaskResponded>() {
                            Ok(event) => {
                                if task_response_logs.send(event).await.is_err() {
                                    log::error!("Failed to send task response event");
                                }
                            }
                            Err(e) => log::error!("Failed to parse task response event: {:?}", e),
                        };
                    }
                }
            }
        });

        Ok(subscription)
    }
}
