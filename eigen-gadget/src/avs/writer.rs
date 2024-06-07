use std::str::FromStr;

use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::network::{Ethereum, EthereumSigner};
use alloy_provider::{Provider, WalletProvider};
use alloy_rpc_types::{Log, TransactionReceipt};
use alloy_transport::Transport;
use async_trait::async_trait;
use eigen_utils::crypto::bls::G1Point;
use eigen_utils::types::AvsError;

use super::IncredibleSquaringTaskManager::{Task, TaskResponse, TaskResponseMetadata};
use super::{AvsManagersBindings, IncredibleSquaringTaskManager, SetupConfig, SignerType};

#[async_trait]
pub trait AvsWriterTrait: Send + Sync {
    async fn send_new_task_number_to_square(
        &self,
        num_to_square: U256,
        quorum_threshold_percentage: u32,
        quorum_numbers: Bytes,
    ) -> Result<(Task, u32), AvsError>;

    async fn raise_challenge(
        &self,
        task: Task,
        task_response: TaskResponse,
        task_response_metadata: TaskResponseMetadata,
        pubkeys_of_non_signing_operators: Vec<G1Point>,
    ) -> Result<TransactionReceipt, AvsError>;

    async fn send_aggregated_response(
        &self,
        task: Task,
        task_response: TaskResponse,
        non_signer_stakes_and_signature: IncredibleSquaringTaskManager::NonSignerStakesAndSignature,
    ) -> Result<TransactionReceipt, AvsError>;
}

pub struct AvsWriter<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    avs_contract_bindings: AvsManagersBindings<T, P>,
    eth_client: P,
}

impl<T, P> AvsWriter<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub fn new(avs_contract_bindings: AvsManagersBindings<T, P>, eth_client: P) -> Self {
        Self {
            avs_contract_bindings,
            eth_client,
        }
    }

    pub async fn from_config(config: &SetupConfig<T, P>) -> Result<Self, AvsError> {
        let avs_contract_bindings = AvsManagersBindings::new(
            config.registry_coordinator_addr,
            config.operator_state_retriever_addr,
            config.eth_client.clone(),
        )
        .await?;

        Ok(Self {
            avs_contract_bindings: avs_contract_bindings,
            eth_client: config.eth_client.clone(),
        })
    }

    pub async fn build(
        registry_coordinator_addr: &str,
        operator_state_retriever_addr: &str,
        eth_client: P,
    ) -> Result<Self, AvsError> {
        let avs_contract_bindings = AvsManagersBindings::new(
            Address::from_str(registry_coordinator_addr).unwrap_or_default(),
            Address::from_str(operator_state_retriever_addr).unwrap_or_default(),
            eth_client.clone(),
        )
        .await?;

        Ok(Self {
            avs_contract_bindings: avs_contract_bindings,
            eth_client,
        })
    }
}

#[async_trait]
impl<T, P> AvsWriterTrait for AvsWriter<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    async fn send_new_task_number_to_square(
        &self,
        num_to_square: U256,
        quorum_threshold_percentage: u32,
        quorum_numbers: Bytes,
    ) -> Result<(Task, u32), AvsError> {
        let receipt = self
            .avs_contract_bindings
            .task_manager
            .createNewTask(num_to_square, quorum_threshold_percentage, quorum_numbers)
            .send()
            .await?
            .get_receipt()
            .await?;

        receipt
            .inner
            .logs()
            .iter()
            .find_map(|log| {
                log.log_decode::<IncredibleSquaringTaskManager::NewTaskCreated>()
                    .ok()
            })
            .map(|log| (log.inner.task, log.inner.taskIndex))
            .ok_or(AvsError::InvalidLogDecodingError(
                "NewTaskCreated event not found".to_string(),
            ))
    }

    async fn raise_challenge(
        &self,
        task: Task,
        task_response: TaskResponse,
        task_response_metadata: TaskResponseMetadata,
        pubkeys_of_non_signing_operators: Vec<G1Point>,
    ) -> Result<TransactionReceipt, AvsError> {
        self.avs_contract_bindings
            .task_manager
            .raiseAndResolveChallenge(
                task,
                task_response,
                task_response_metadata,
                pubkeys_of_non_signing_operators
                    .iter()
                    .map(|pt| IncredibleSquaringTaskManager::G1Point { X: pt.x, Y: pt.y })
                    .collect(),
            )
            .send()
            .await?
            .get_receipt()
            .await
            .map_err(|e| AvsError::from(e))
    }

    async fn send_aggregated_response(
        &self,
        task: Task,
        task_response: TaskResponse,
        non_signer_stakes_and_signature: IncredibleSquaringTaskManager::NonSignerStakesAndSignature,
    ) -> Result<TransactionReceipt, AvsError> {
        self.avs_contract_bindings
            .task_manager
            .respondToTask(task, task_response, non_signer_stakes_and_signature)
            .send()
            .await?
            .get_receipt()
            .await
            .map_err(|e| AvsError::from(e))
    }
}

pub struct TaskManager<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub bindings: AvsManagersBindings<T, P>,
}

impl<T, P> TaskManager<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub async fn new(
        registry_coordinator_addr: Address,
        operator_state_retriever_addr: Address,
        eth_client: P,
    ) -> Result<Self, AvsError> {
        let bindings = AvsManagersBindings::new(
            registry_coordinator_addr,
            operator_state_retriever_addr,
            eth_client,
        )
        .await?;

        Ok(Self { bindings })
    }

    pub async fn create_new_task(
        &self,
        num_to_square: U256,
        quorum_threshold_percentage: u32,
        quorum_numbers: Bytes,
    ) -> Result<TransactionReceipt, AvsError> {
        self.bindings
            .task_manager
            .createNewTask(num_to_square, quorum_threshold_percentage, quorum_numbers)
            .send()
            .await?
            .get_receipt()
            .await
            .map_err(|e| AvsError::from(e))
    }

    pub async fn parse_new_task_created(
        &self,
        log: &Log,
    ) -> Result<Log<IncredibleSquaringTaskManager::NewTaskCreated>, AvsError> {
        Ok(log
            .log_decode::<IncredibleSquaringTaskManager::NewTaskCreated>()
            .map_err(|e| AvsError::from(e))?)
    }

    pub async fn raise_and_resolve_challenge(
        &self,
        task: Task,
        task_response: TaskResponse,
        task_response_metadata: TaskResponseMetadata,
        pubkeys_of_non_signing_operators: Vec<G1Point>,
    ) -> Result<TransactionReceipt, AvsError> {
        self.bindings
            .task_manager
            .raiseAndResolveChallenge(
                task,
                task_response,
                task_response_metadata,
                pubkeys_of_non_signing_operators
                    .iter()
                    .map(|pt| IncredibleSquaringTaskManager::G1Point { X: pt.x, Y: pt.y })
                    .collect(),
            )
            .send()
            .await?
            .get_receipt()
            .await
            .map_err(|e| AvsError::from(e))
    }

    pub async fn respond_to_task(
        &self,
        task: Task,
        task_response: TaskResponse,
        non_signer_stakes_and_signature: IncredibleSquaringTaskManager::NonSignerStakesAndSignature,
    ) -> Result<TransactionReceipt, AvsError> {
        self.bindings
            .task_manager
            .respondToTask(task, task_response, non_signer_stakes_and_signature)
            .send()
            .await?
            .get_receipt()
            .await
            .map_err(|e| AvsError::from(e))
    }
}
