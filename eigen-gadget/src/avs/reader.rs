use super::{AvsManagersBindings, Erc20Mock, IncredibleSquaringTaskManager, SetupConfig};
use alloy_primitives::{Address, Bytes, FixedBytes};
use alloy_provider::{network::Ethereum, Provider, ProviderBuilder, RootProvider};
use alloy_transport::Transport;
use async_trait::async_trait;
use eigen_utils::{avs_registry::reader::AvsRegistryChainReader, types::AvsError};
use std::{str::FromStr, sync::Arc};

#[async_trait]
pub trait AvsReaderer<T, P>: Send + Sync
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    async fn check_signatures(
        &self,
        msg_hash: FixedBytes<32>,
        quorum_numbers: Bytes,
        reference_block_number: u32,
        non_signer_stakes_and_signature: IncredibleSquaringTaskManager::NonSignerStakesAndSignature,
    ) -> Result<IncredibleSquaringTaskManager::QuorumStakeTotals, AvsError>;

    async fn get_erc20_mock(
        &self,
        token_addr: Address,
    ) -> Result<Erc20Mock::Erc20MockInstance<T, P>, AvsError>;
}

pub struct AvsReader<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    avs_registry_reader: AvsRegistryChainReader<T, P>,
    avs_service_bindings: AvsManagersBindings<T, P>,
    eth_client: P,
}

impl<T, P> AvsReader<T, P>
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
        let avs_managers_bindings = AvsManagersBindings::new(
            registry_coordinator_addr,
            operator_state_retriever_addr,
            eth_client.clone(),
        )
        .await?;

        let avs_registry_reader = AvsRegistryChainReader::build(
            registry_coordinator_addr,
            operator_state_retriever_addr,
            eth_client.clone(),
        )
        .await;

        Ok(Self {
            avs_registry_reader: avs_registry_reader,
            avs_service_bindings: avs_managers_bindings,
            eth_client: eth_client,
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
impl<T, P> AvsReaderer<T, P> for AvsReader<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    async fn check_signatures(
        &self,
        msg_hash: FixedBytes<32>,
        quorum_numbers: Bytes,
        reference_block_number: u32,
        non_signer_stakes_and_signature: IncredibleSquaringTaskManager::NonSignerStakesAndSignature,
    ) -> Result<IncredibleSquaringTaskManager::QuorumStakeTotals, AvsError> {
        let stake_totals_per_quorum = self
            .avs_service_bindings
            .task_manager
            .checkSignatures(
                msg_hash,
                quorum_numbers,
                reference_block_number,
                non_signer_stakes_and_signature,
            )
            .call()
            .await
            .map(|x| x._0)?;
        Ok(stake_totals_per_quorum)
    }

    async fn get_erc20_mock(
        &self,
        token_addr: Address,
    ) -> Result<Erc20Mock::Erc20MockInstance<T, P>, AvsError> {
        let erc20_mock = self.avs_service_bindings.get_erc20_mock(token_addr).await?;
        Ok(erc20_mock)
    }
}
