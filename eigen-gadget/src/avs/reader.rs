use alloy_primitives::{Address as AlloyAddress, Bytes as AlloyBytes, U256};
use alloy_solidity_abi::Token;
use async_trait::async_trait;
use tokio::sync::Mutex;
use std::{sync::Arc, error::Error};


#[async_trait]
pub trait AvsReaderer: Send + Sync {
    async fn check_signatures(
        &self,
        msg_hash: [u8; 32],
        quorum_numbers: Vec<u8>,
        reference_block_number: u32,
        non_signer_stakes_and_signature: IncredibleSquaringTaskManager::NonSignerStakesAndSignature,
    ) -> Result<IncredibleSquaringTaskManager::QuorumStakeTotals, Box<dyn Error + Send + Sync>>;

    async fn get_erc20_mock(
        &self,
        token_addr: AlloyAddress,
    ) -> Result<Erc20Mock, Box<dyn Error + Send + Sync>>;
}

pub struct AvsReader {
    avs_registry_reader: Arc<dyn AvsRegistryReader>,
    avs_service_bindings: Arc<AvsManagersBindings>,
    logger: Arc<Logger>,
}

impl AvsReader {
    pub async fn from_config(config: &Config) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Self::new(
            config.registry_coordinator_addr,
            config.operator_state_retriever_addr,
            Arc::new(EthClient::new(&config.eth_http_url).await?),
            Arc::new(Logger::new(&config.logger)),
        )
        .await
    }

    pub async fn new(
        registry_coordinator_addr: AlloyAddress,
        operator_state_retriever_addr: AlloyAddress,
        eth_http_client: Arc<EthClient>,
        logger: Arc<Logger>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let avs_managers_bindings = AvsManagersBindings::new(
            registry_coordinator_addr,
            operator_state_retriever_addr,
            eth_http_client.clone(),
            logger.clone(),
        )
        .await?;

        let avs_registry_reader = AvsRegistryReader::new(
            registry_coordinator_addr,
            operator_state_retriever_addr,
            eth_http_client,
            logger.clone(),
        )
        .await?;

        Ok(Self {
            avs_registry_reader: Arc::new(avs_registry_reader),
            avs_service_bindings: Arc::new(avs_managers_bindings),
            logger,
        })
    }
}

#[async_trait]
impl AvsReaderer for AvsReader {
    async fn check_signatures(
        &self,
        msg_hash: [u8; 32],
        quorum_numbers: Vec<u8>,
        reference_block_number: u32,
        non_signer_stakes_and_signature: IncredibleSquaringTaskManager::NonSignerStakesAndSignature,
    ) -> Result<IncredibleSquaringTaskManager::QuorumStakeTotals, Box<dyn Error + Send + Sync>> {
        let stake_totals_per_quorum = self
            .avs_service_bindings
            .task_manager
            .check_signatures(
                msg_hash,
                quorum_numbers,
                reference_block_number,
                non_signer_stakes_and_signature,
            )
            .await?;
        Ok(stake_totals_per_quorum)
    }

    async fn get_erc20_mock(
        &self,
        token_addr: AlloyAddress,
    ) -> Result<Erc20Mock, Box<dyn Error + Send + Sync>> {
        let erc20_mock = self
            .avs_service_bindings
            .get_erc20_mock(token_addr)
            .await?;
        Ok(erc20_mock)
    }
}

pub struct AvsManagersBindings {
    pub task_manager: IncredibleSquaringTaskManager,
    eth_client: Arc<EthClient>,
    logger: Arc<Logger>,
}

impl AvsManagersBindings {
    pub async fn new(
        registry_coordinator_addr: AlloyAddress,
        operator_state_retriever_addr: AlloyAddress,
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

    pub async fn get_erc20_mock(
        &self,
        token_addr: AlloyAddress,
    ) -> Result<Erc20Mock, Box<dyn Error + Send + Sync>> {
        let erc20_mock = Erc20Mock::new(token_addr, self.eth_client.clone()).await?;
        Ok(erc20_mock)
    }
}

// Configuration struct as an example. Adapt as needed.
pub struct Config {
    pub registry_coordinator_addr: AlloyAddress,
    pub operator_state_retriever_addr: AlloyAddress,
    pub eth_http_url: String,
    pub logger: String,
}
