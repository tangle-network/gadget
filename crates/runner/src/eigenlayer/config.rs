use crate::config::{BlueprintSettings, ProtocolSettingsT};
use crate::error::ConfigError;
use alloy_primitives::{Address, address};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
/// The contract addresses used for EigenLayer Blueprint AVSs
///
/// The default values of these contracts are the addresses for our testing environment.
pub struct EigenlayerProtocolSettings {
    /// The address of the registry coordinator contract
    pub registry_coordinator_address: Address,
    /// The address of the operator state retriever contract
    pub operator_state_retriever_address: Address,
    /// The address of the operator registry contract
    pub delegation_manager_address: Address,
    /// The address of the Service Manager contract
    pub service_manager_address: Address,
    /// The address of the Stake Registry contract
    pub stake_registry_address: Address,
    /// The address of the strategy manager contract
    pub strategy_manager_address: Address,
    /// The address of the avs registry contract
    pub avs_directory_address: Address,
    pub rewards_coordinator_address: Address,
}

impl ProtocolSettingsT for EigenlayerProtocolSettings {
    type Settings = Self;

    fn load(settings: BlueprintSettings) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(EigenlayerProtocolSettings {
            registry_coordinator_address: settings
                .registry_coordinator
                .ok_or(ConfigError::MissingEigenlayerContractAddresses)?,
            operator_state_retriever_address: settings
                .operator_state_retriever
                .ok_or(ConfigError::MissingEigenlayerContractAddresses)?,
            delegation_manager_address: settings
                .delegation_manager
                .ok_or(ConfigError::MissingEigenlayerContractAddresses)?,
            service_manager_address: settings
                .service_manager
                .ok_or(ConfigError::MissingEigenlayerContractAddresses)?,
            stake_registry_address: settings
                .stake_registry
                .ok_or(ConfigError::MissingEigenlayerContractAddresses)?,
            strategy_manager_address: settings
                .strategy_manager
                .ok_or(ConfigError::MissingEigenlayerContractAddresses)?,
            avs_directory_address: settings
                .avs_directory
                .ok_or(ConfigError::MissingEigenlayerContractAddresses)?,
            rewards_coordinator_address: settings
                .rewards_coordinator
                .ok_or(ConfigError::MissingEigenlayerContractAddresses)?,
        })
    }

    fn protocol(&self) -> &'static str {
        "eigenlayer"
    }

    fn settings(&self) -> &Self::Settings {
        self
    }
}

impl Default for EigenlayerProtocolSettings {
    fn default() -> Self {
        Self {
            registry_coordinator_address: address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690"),
            operator_state_retriever_address: address!("1613beb3b2c4f22ee086b2b38c1476a3ce7f78e8"),
            delegation_manager_address: address!("dc64a140aa3e981100a9beca4e685f962f0cf6c9"),
            service_manager_address: address!("0000000000000000000000000000000000000000"),
            stake_registry_address: address!("0000000000000000000000000000000000000000"),
            strategy_manager_address: address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707"),
            avs_directory_address: address!("0000000000000000000000000000000000000000"),
            rewards_coordinator_address: address!("0000000000000000000000000000000000000000"),
        }
    }
}
