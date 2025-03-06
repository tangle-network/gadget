use crate::config::{BlueprintSettings, ProtocolSettingsT};
use crate::error::ConfigError;
use alloy_primitives::{Address, address};
use serde::{Deserialize, Serialize};
use std::error::Error;

/// The contract addresses used for EigenLayer Blueprint AVSs
///
/// The default values of these contracts are the addresses for our testing environment.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct EigenlayerProtocolSettings {
    /// The address of the allocation manager contract
    pub allocation_manager_address: Address,
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
    /// The address of the rewards coordinator contract
    pub rewards_coordinator_address: Address,
    /// The address of the permission controller contract
    pub permission_controller_address: Address,
}

impl ProtocolSettingsT for EigenlayerProtocolSettings {
    type Settings = Self;

    fn load(settings: BlueprintSettings) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(EigenlayerProtocolSettings {
            allocation_manager_address: settings
                .allocation_manager
                .ok_or(ConfigError::MissingEigenlayerContractAddresses)?,
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
            permission_controller_address: settings
                .permission_controller
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
            allocation_manager_address: address!("67d269191c92caf3cd7723f116c85e6e9bf55933"),
            registry_coordinator_address: address!("4c4a2f8c81640e47606d3fd77b353e87ba015584"),
            operator_state_retriever_address: address!("1429859428c0abc9c2c47c8ee9fbaf82cfa0f20f"),
            delegation_manager_address: address!("a85233c63b9ee964add6f2cffe00fd84eb32338f"),
            service_manager_address: address!("0000000000000000000000000000000000000000"), // Depends on AVS
            stake_registry_address: address!("922d6956c99e12dfeb3224dea977d0939758a1fe"), // Differs when using ECDSA Base
            strategy_manager_address: address!("09635f643e140090a9a8dcd712ed6285858cebef"),
            avs_directory_address: address!("7a2088a1bfc9d81c55368ae168c2c02570cb814f"),
            rewards_coordinator_address: address!("c3e53f4d16ae77db1c982e75a937b9f60fe63690"),
            permission_controller_address: address!("0x4a679253410272dd5232b3ff7cf5dbb88f295319"),
        }
    }
}
