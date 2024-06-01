use crate::types::*;

use alloy_network::Ethereum;
use alloy_network::EthereumSigner;
use alloy_network::Network;
use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use alloy_rpc_types::TransactionReceipt;
use alloy_transport::Transport;
use async_trait::async_trait;
use eigen_contracts::AVSDirectory;
use eigen_contracts::DelegationManager;
use eigen_contracts::ISlasher;
use eigen_contracts::StrategyManager;


use super::reader::ElChainReader;
use super::reader::ElReader;
// use crate::logging::Logger;

#[async_trait]
pub trait ElWriter: Send + Sync {
    async fn register_as_operator(
        &self,
        operator: Operator,
    ) -> Result<TransactionReceipt, AvsError>;
    async fn update_operator_details(
        &self,
        operator: Operator,
    ) -> Result<TransactionReceipt, AvsError>;
    async fn deposit_erc20_into_strategy(
        &self,
        strategy_addr: Address,
        amount: U256,
    ) -> Result<TransactionReceipt, AvsError>;
}

pub struct ElChainWriter<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Copy + 'static,
{
    slasher: ISlasher::ISlasherInstance<T, P>,
    delegation_manager: DelegationManager::DelegationManagerInstance<T, P>,
    strategy_manager: StrategyManager::StrategyManagerInstance<T, P>,
    el_chain_reader: ElChainReader<T, P>,
    eth_client: P,
    // logger: Logger,
    tx_mgr: EthereumSigner,
}

impl<T, P> ElChainWriter<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Copy + 'static,
{
    pub fn new(
        slasher: ISlasher::ISlasherInstance<T, P>,
        delegation_manager: DelegationManager::DelegationManagerInstance<T, P>,
        strategy_manager: StrategyManager::StrategyManagerInstance<T, P>,
        el_chain_reader: ElChainReader<T, P>,
        eth_client: P,
        // logger: Logger,
        tx_mgr: EthereumSigner,
    ) -> Self {
        Self {
            slasher,
            delegation_manager,
            strategy_manager,
            el_chain_reader,
            eth_client,
            // logger,
            tx_mgr,
        }
    }

    pub async fn build(
        delegation_manager_addr: Address,
        avs_directory_addr: Address,
        strategy_manager_addr: Address,
        eth_client: P,
        // logger: Logger,
        tx_mgr: EthereumSigner,
    ) -> Result<Self, AvsError> {
        let delegation_manager = DelegationManager::new(delegation_manager_addr, eth_client);
        let slash_addr = delegation_manager.slasher().call().await.map(|a| a._0)?;
        let slasher = ISlasher::new(slash_addr, eth_client);
        let strategy_manager = StrategyManager::new(strategy_manager_addr, eth_client);
        let el_chain_reader = ElChainReader::build(
            delegation_manager_addr,
            avs_directory_addr,
            strategy_manager_addr,
            // logger.clone(),
            eth_client,
        )
        .await?;
        Ok(Self::new(
            slasher,
            delegation_manager,
            strategy_manager,
            el_chain_reader,
            eth_client,
            // logger,
            tx_mgr,
        ))
    }
}

#[async_trait]
impl<T, P> ElWriter for ElChainWriter<T, P>
where
    T: Transport + Clone,
    P: Provider<T, Ethereum> + Copy,
{
    async fn register_as_operator(
        &self,
        operator: Operator,
    ) -> Result<TransactionReceipt, AvsError> {
        log::info!(
            "registering operator {} to EigenLayer",
            operator.address
        );

        let op_details = DelegationManager::OperatorDetails {
            earningsReceiver: operator.earnings_receiver_address,
            stakerOptOutWindowBlocks: operator.staker_opt_out_window_blocks,
            delegationApprover: operator.delegation_approver_address,
        };

        let receipt = self
            .delegation_manager
            .registerAsOperator(op_details, operator.metadata_url)
            .send()
            .await?
            .get_receipt()
            .await?;
    
        log::info!(
            "Successfully registered operator to EigenLayer, txHash: {}",
            receipt.transaction_hash
        );

        Ok(receipt)
    }

    async fn update_operator_details(
        &self,
        operator: Operator,
    ) -> Result<TransactionReceipt, AvsError> {
        log::info!(
            "updating operator details of operator {} to EigenLayer",
            operator.address
        );

        let op_details = DelegationManager::OperatorDetails {
            earningsReceiver: operator.earnings_receiver_address,
            stakerOptOutWindowBlocks: operator.staker_opt_out_window_blocks,
            delegationApprover: operator.delegation_approver_address,
        };

        let receipt = self
            .delegation_manager
            .modifyOperatorDetails(op_details)
            .send()
            .await?
            .get_receipt()
            .await?;

        log::info!(
            "successfully updated operator metadata URI, txHash: {}",
            receipt.transaction_hash
        );

        let receipt = self
            .delegation_manager
            .updateOperatorMetadataURI(operator.metadata_url)
            .send()
            .await?
            .get_receipt()
            .await?;

        log::info!(
            "successfully updated operator details, txHash: {}",
            receipt.transaction_hash
        );

        Ok(receipt)
    }

    async fn deposit_erc20_into_strategy(
        &self,
        strategy_addr: Address,
        amount: U256,
    ) -> Result<TransactionReceipt, AvsError> {
        log::info!(
            "depositing {} tokens into strategy {}",
            amount, strategy_addr
        );

        let (_, underlying_token_contract, underlying_token_addr) = self
            .el_chain_reader
            .get_strategy_and_underlying_erc20_token(strategy_addr)
            .await?;
        let receipt = underlying_token_contract
            .approve(*self.strategy_manager.address(), amount)
            .send()
            .await?
            .get_receipt()
            .await?;
        
        log::info!(
            "approved {} tokens for deposit into strategy {} with txHash: {}",
            amount, strategy_addr, receipt.transaction_hash
        );

        let receipt = self
            .strategy_manager
            .depositIntoStrategy(strategy_addr, underlying_token_addr, amount)
            .send()
            .await?
            .get_receipt()
            .await?;

        log::info!(
            "deposited {} into strategy {}",
            amount, strategy_addr
        );

        Ok(receipt)
    }
}
