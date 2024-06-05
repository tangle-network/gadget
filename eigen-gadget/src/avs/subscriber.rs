use alloy_primitives::Address;
use alloy_solidity_abi::Token;
use async_trait::async_trait;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use alloy::event::{SubscriptionStream, RawLog};

use crate::{
    clients::{eth::EthClient},
    contracts::{cstaskmanager::{IncredibleSquaringTaskManager, ContractIncredibleSquaringTaskManagerNewTaskCreated, ContractIncredibleSquaringTaskManagerTaskResponded}},
    logging::Logger,
    config::Config,
};

#[async_trait]
pub trait AvsSubscriberer: Send + Sync {
    async fn subscribe_to_new_tasks(
        &self,
        new_task_created_chan: tokio::sync::mpsc::Sender<ContractIncredibleSquaringTaskManagerNewTaskCreated>,
    ) -> Result<SubscriptionStream<RawLog>, Box<dyn Error + Send + Sync>>;

    async fn subscribe_to_task_responses(
        &self,
        task_response_logs: tokio::sync::mpsc::Sender<ContractIncredibleSquaringTaskManagerTaskResponded>,
    ) -> Result<SubscriptionStream<RawLog>, Box<dyn Error + Send + Sync>>;

    async fn parse_task_responded(
        &self,
        raw_log: RawLog,
    ) -> Result<ContractIncredibleSquaringTaskManagerTaskResponded, Box<dyn Error + Send + Sync>>;
}

pub struct AvsSubscriber {
    avs_contract_bindings: Arc<AvsManagersBindings>,
    logger: Arc<Logger>,
}

impl AvsSubscriber {
    pub async fn from_config(config: &Config) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Self::new(
            config.registry_coordinator_addr,
            config.operator_state_retriever_addr,
            Arc::new(EthClient::new(&config.eth_ws_url).await?),
            Arc::new(Logger::new(&config.logger)),
        )
        .await
    }

    pub async fn new(
        registry_coordinator_addr: Address,
        bls_operator_state_retriever_addr: Address,
        eth_client: Arc<EthClient>,
        logger: Arc<Logger>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let avs_contract_bindings = AvsManagersBindings::new(
            registry_coordinator_addr,
            bls_operator_state_retriever_addr,
            eth_client,
            logger.clone(),
        )
        .await?;

        Ok(Self {
            avs_contract_bindings: Arc::new(avs_contract_bindings),
            logger,
        })
    }
}

#[async_trait]
impl AvsSubscriberer for AvsSubscriber {
    async fn subscribe_to_new_tasks(
        &self,
        new_task_created_chan: tokio::sync::mpsc::Sender<ContractIncredibleSquaringTaskManagerNewTaskCreated>,
    ) -> Result<SubscriptionStream<RawLog>, Box<dyn Error + Send + Sync>> {
        let mut stream = self
            .avs_contract_bindings
            .task_manager
            .watch_new_task_created()
            .await?;

        let avs_contract_bindings = self.avs_contract_bindings.clone();
        let logger = self.logger.clone();

        tokio::spawn(async move {
            while let Some(raw_log) = stream.next().await {
                match avs_contract_bindings.task_manager.parse_new_task_created(raw_log.clone()).await {
                    Ok(event) => {
                        if new_task_created_chan.send(event).await.is_err() {
                            logger.error("Failed to send new task created event");
                        }
                    }
                    Err(e) => logger.error(&format!("Failed to parse new task created event: {:?}", e)),
                }
            }
        });

        Ok(stream)
    }

    async fn subscribe_to_task_responses(
        &self,
        task_response_logs: tokio::sync::mpsc::Sender<ContractIncredibleSquaringTaskManagerTaskResponded>,
    ) -> Result<SubscriptionStream<RawLog>, Box<dyn Error + Send + Sync>> {
        let mut stream = self
            .avs_contract_bindings
            .task_manager
            .watch_task_responded()
            .await?;

        let avs_contract_bindings = self.avs_contract_bindings.clone();
        let logger = self.logger.clone();

        tokio::spawn(async move {
            while let Some(raw_log) = stream.next().await {
                match avs_contract_bindings.task_manager.parse_task_responded(raw_log.clone()).await {
                    Ok(event) => {
                        if task_response_logs.send(event).await.is_err() {
                            logger.error("Failed to send task response event");
                        }
                    }
                    Err(e) => logger.error(&format!("Failed to parse task response event: {:?}", e)),
                }
            }
        });

        Ok(stream)
    }

    async fn parse_task_responded(
        &self,
        raw_log: RawLog,
    ) -> Result<ContractIncredibleSquaringTaskManagerTaskResponded, Box<dyn Error + Send + Sync>> {
        self.avs_contract_bindings
            .task_manager
            .parse_task_responded(raw_log)
            .await
    }
}

pub struct AvsManagersBindings {
    pub task_manager: IncredibleSquaringTaskManager,
    eth_client: Arc<EthClient>,
    logger: Arc<Logger>,
}

impl AvsManagersBindings {
    pub async fn new(
        registry_coordinator_addr: Address,
        operator_state_retriever_addr: Address,
        eth_client: Arc<EthClient>,
        logger: Arc<Logger>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let task_manager = IncredibleSquaringTaskManager::new(registry_coordinator_addr, eth_client.clone()).await?;
        Ok(Self {
            task_manager,
            eth_client,
            logger,
        })
    }
}

// Configuration struct as an example. Adapt as needed.
pub struct Config {
    pub registry_coordinator_addr: Address,
    pub operator_state_retriever_addr: Address,
    pub eth_ws_url: String,
    pub logger: String,
}
