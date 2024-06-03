use crate::types::*;

use alloy_network::{Ethereum, Network};
use alloy_primitives::FixedBytes;
use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use alloy_transport::Transport;
use async_trait::async_trait;
use eigen_contracts::AVSDirectory;
use eigen_contracts::DelegationManager;
use eigen_contracts::ISlasher;
use eigen_contracts::IStrategy;
use eigen_contracts::StrategyManager;
use eigen_contracts::IERC20;
// use crate::logging::Logger;

#[async_trait]
pub trait ElReader<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    async fn is_operator_registered(&self, operator: &Operator) -> Result<bool, AvsError>;
    async fn get_operator_details(&self, operator: &Operator) -> Result<Operator, AvsError>;
    async fn get_strategy_and_underlying_token(
        &self,
        strategy_addr: Address,
    ) -> Result<(IStrategy::IStrategyInstance<T, P>, Address), AvsError>;
    async fn get_strategy_and_underlying_erc20_token(
        &self,
        strategy_addr: Address,
    ) -> Result<
        (
            IStrategy::IStrategyInstance<T, P>,
            IERC20::IERC20Instance<T, P>,
            Address,
        ),
        AvsError,
    >;
    async fn service_manager_can_slash_operator_until_block(
        &self,
        operator_addr: Address,
        service_manager_addr: Address,
    ) -> Result<u32, AvsError>;
    async fn operator_is_frozen(&self, operator_addr: Address) -> Result<bool, AvsError>;
    async fn get_operator_shares_in_strategy(
        &self,
        operator_addr: Address,
        strategy_addr: Address,
    ) -> Result<U256, AvsError>;
    async fn calculate_delegation_approval_digest_hash(
        &self,
        staker: Address,
        operator: Address,
        delegation_approver: Address,
        approver_salt: FixedBytes<32>,
        expiry: U256,
    ) -> Result<FixedBytes<32>, AvsError>;
    async fn calculate_operator_avs_registration_digest_hash(
        &self,
        operator: Address,
        avs: Address,
        salt: FixedBytes<32>,
        expiry: U256,
    ) -> Result<FixedBytes<32>, AvsError>;
}

pub struct ElChainReader<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    // logger: Logger,
    slasher: ISlasher::ISlasherInstance<T, P>,
    delegation_manager: DelegationManager::DelegationManagerInstance<T, P>,
    strategy_manager: StrategyManager::StrategyManagerInstance<T, P>,
    avs_directory: AVSDirectory::AVSDirectoryInstance<T, P>,
    eth_client: P,
}

impl<T, P> ElChainReader<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    pub fn new(
        slasher: ISlasher::ISlasherInstance<T, P>,
        delegation_manager: DelegationManager::DelegationManagerInstance<T, P>,
        strategy_manager: StrategyManager::StrategyManagerInstance<T, P>,
        avs_directory: AVSDirectory::AVSDirectoryInstance<T, P>,
        // logger: Logger,
        eth_client: P,
    ) -> Self {
        Self {
            slasher,
            delegation_manager,
            strategy_manager,
            avs_directory,

            eth_client,
        }
    }

    pub async fn build(
        delegation_manager_addr: Address,
        avs_directory_addr: Address,
        strategy_manager_addr: Address,
        eth_client: P,
        // logger: Logger,
    ) -> Result<Self, AvsError> {
        let delegation_manager =
            DelegationManager::new(delegation_manager_addr, eth_client.clone());
        let slash_addr = delegation_manager.slasher().call().await.map(|a| a._0)?;
        let slasher = ISlasher::new(slash_addr, eth_client.clone());
        let strategy_manager = StrategyManager::new(strategy_manager_addr, eth_client.clone());
        let avs_directory = AVSDirectory::new(avs_directory_addr, eth_client.clone());
        Ok(Self::new(
            slasher,
            delegation_manager,
            strategy_manager,
            avs_directory,
            eth_client,
        ))
    }
}

#[async_trait]
impl<T, P> ElReader<T, P> for ElChainReader<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Clone,
{
    async fn is_operator_registered(&self, operator: &Operator) -> Result<bool, AvsError> {
        let is_operator = self
            .delegation_manager
            .isOperator(operator.address)
            .call()
            .await
            .map(|is_operator| is_operator._0)?;
        Ok(is_operator)
    }

    async fn get_operator_details(&self, operator: &Operator) -> Result<Operator, AvsError> {
        let details = self
            .delegation_manager
            .operatorDetails(operator.address)
            .call()
            .await
            .map(|details| details._0)?;
        Ok(Operator {
            address: operator.address,
            earnings_receiver_address: details.earningsReceiver,
            staker_opt_out_window_blocks: details.stakerOptOutWindowBlocks,
            delegation_approver_address: details.delegationApprover,
            ..operator.clone()
        })
    }

    async fn get_strategy_and_underlying_token(
        &self,
        strategy_addr: Address,
    ) -> Result<(IStrategy::IStrategyInstance<T, P>, Address), AvsError> {
        let contract_strategy = IStrategy::new(strategy_addr, self.eth_client);
        let underlying_token_addr = contract_strategy
            .underlyingToken()
            .call()
            .await
            .map(|addr| addr._0)?;
        Ok((contract_strategy, underlying_token_addr))
    }

    async fn get_strategy_and_underlying_erc20_token(
        &self,
        strategy_addr: Address,
    ) -> Result<
        (
            IStrategy::IStrategyInstance<T, P>,
            IERC20::IERC20Instance<T, P>,
            Address,
        ),
        AvsError,
    > {
        let contract_strategy = IStrategy::new(strategy_addr, self.eth_client);
        let underlying_token_addr = contract_strategy
            .underlyingToken()
            .call()
            .await
            .map(|addr| addr._0)?;
        let contract_underlying_token = IERC20::new(underlying_token_addr, self.eth_client);
        Ok((
            contract_strategy,
            contract_underlying_token,
            underlying_token_addr,
        ))
    }

    async fn service_manager_can_slash_operator_until_block(
        &self,
        operator_addr: Address,
        service_manager_addr: Address,
    ) -> Result<u32, AvsError> {
        let until_block = self
            .slasher
            .contractCanSlashOperatorUntilBlock(operator_addr, service_manager_addr)
            .call()
            .await
            .map(|block| block._0)?;
        Ok(until_block)
    }

    async fn operator_is_frozen(&self, operator_addr: Address) -> Result<bool, AvsError> {
        let is_frozen = self
            .slasher
            .isFrozen(operator_addr)
            .call()
            .await
            .map(|frozen| frozen._0)?;
        Ok(is_frozen)
    }

    async fn get_operator_shares_in_strategy(
        &self,
        operator_addr: Address,
        strategy_addr: Address,
    ) -> Result<U256, AvsError> {
        let shares = self
            .delegation_manager
            .operatorShares(operator_addr, strategy_addr)
            .call()
            .await
            .map(|shares| shares._0)?;
        Ok(shares)
    }

    async fn calculate_delegation_approval_digest_hash(
        &self,
        staker: Address,
        operator: Address,
        delegation_approver: Address,
        approver_salt: FixedBytes<32>,
        expiry: U256,
    ) -> Result<FixedBytes<32>, AvsError> {
        let digest = self
            .delegation_manager
            .calculateDelegationApprovalDigestHash(
                staker,
                operator,
                delegation_approver,
                approver_salt,
                expiry,
            )
            .call()
            .await
            .map(|digest| digest._0)?;
        Ok(digest)
    }

    async fn calculate_operator_avs_registration_digest_hash(
        &self,
        operator: Address,
        avs: Address,
        salt: FixedBytes<32>,
        expiry: U256,
    ) -> Result<FixedBytes<32>, AvsError> {
        let digest = self
            .avs_directory
            .calculateOperatorAVSRegistrationDigestHash(operator, avs, salt, expiry)
            .call()
            .await
            .map(|digest| digest._0)?;
        Ok(digest)
    }
}
