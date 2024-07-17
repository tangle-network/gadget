use alloy_primitives::keccak256;
use alloy_provider::Provider;
use alloy_pubsub::Subscription;
use alloy_rpc_types::{Filter, Log};
use futures::StreamExt;

use async_trait::async_trait;
use eigen_utils::{types::AvsError, Config};

use super::IncredibleSquaringContractManager;

#[async_trait]
pub trait IncredibleSquaringSubscriber: Send + Sync {
    async fn subscribe_to_new_tasks(&self) -> Result<Subscription<Log>, AvsError>;

    async fn subscribe_to_task_responses(&self) -> Result<Subscription<Log>, AvsError>;
}

#[async_trait]
impl<T: Config> IncredibleSquaringSubscriber for IncredibleSquaringContractManager<T> {
    async fn subscribe_to_new_tasks(&self) -> Result<Subscription<Log>, AvsError> {
        let filter = Filter::new()
            .address(self.task_manager_addr)
            .event("NewTaskCreated");
        let sub = self.eth_client_ws.subscribe_logs(&filter).await?;

        let signature = keccak256("NewTaskCreated(latestTaskNum, newTask)".as_bytes());
        let subscription = self
            .eth_client_ws
            .subscribe_logs(&Filter::new().event_signature(signature))
            .await?;
        // let mut stream = subscription.into_stream().take(5);
        // while let Some(tx) = stream.next().await {
        //     log::info!("{tx:#?}");
        // }

        Ok(subscription)
    }

    async fn subscribe_to_task_responses(&self) -> Result<Subscription<Log>, AvsError> {
        let filter = Filter::new()
            .address(self.task_manager_addr)
            .event("TaskResponded");

        let subscription = self.eth_client_ws.subscribe_logs(&filter).await?;

        Ok(subscription)
    }
}
