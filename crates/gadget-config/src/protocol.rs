use super::*;
#[cfg(any(feature = "eigenlayer", feature = "symbiotic"))]
use alloy_primitives::{address, Address};
use core::fmt::Debug;
use serde::{Deserialize, Serialize};

/// The protocol on which a gadget will be executed.
#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(
    feature = "std",
    derive(clap::ValueEnum),
    clap(rename_all = "lowercase")
)]
pub enum Protocol {
    #[default]
    Tangle,
    Eigenlayer,
    Symbiotic,
}

impl Protocol {
    /// Returns the protocol from the environment variable `PROTOCOL`.
    ///
    /// If the environment variable is not set, it defaults to `Protocol::Tangle`.
    ///
    /// # Errors
    ///
    /// * [`Error::UnsupportedProtocol`] if the protocol is unknown. See [`Protocol`].
    #[cfg(feature = "std")]
    pub fn from_env() -> Result<Self, Error> {
        if let Ok(protocol) = std::env::var("PROTOCOL") {
            return protocol.to_ascii_lowercase().parse::<Protocol>();
        }

        Ok(Protocol::default())
    }

    /// Returns the protocol from the environment variable `PROTOCOL`.
    #[cfg(not(feature = "std"))]
    pub fn from_env() -> Result<Self, Error> {
        Ok(Protocol::default())
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tangle => "tangle",
            Self::Eigenlayer => "eigenlayer",
            Self::Symbiotic => "symbiotic",
        }
    }
}

impl core::fmt::Display for Protocol {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl core::str::FromStr for Protocol {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tangle" => Ok(Self::Tangle),
            "eigenlayer" => Ok(Self::Eigenlayer),
            "symbiotic" => Ok(Self::Symbiotic),
            _ => Err(Error::UnsupportedProtocol(s.to_string())),
        }
    }
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct TangleInstanceSettings {
    /// The blueprint ID for the Tangle blueprint
    pub blueprint_id: u64,
    /// The service ID for the Tangle blueprint instance
    ///
    /// Note: This will be `None` in case this gadget is running in Registration Mode.
    pub service_id: Option<u64>,
}

#[cfg(feature = "eigenlayer")]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct EigenlayerContractAddresses {
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

#[cfg(feature = "eigenlayer")]
impl Default for EigenlayerContractAddresses {
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

#[cfg(feature = "symbiotic")]
#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct SymbioticContractAddresses {
    /// The address of the operator registry contract
    pub operator_registry_address: Address,
    /// The address of the network registry contract
    pub network_registry_address: Address,
    /// The address of the base delegator contract
    pub base_delegator_address: Address,
    /// The address of the network opt-in service contract
    pub network_opt_in_service_address: Address,
    /// The address of the vault opt-in service contract
    pub vault_opt_in_service_address: Address,
    /// The address of the slasher contract
    pub slasher_address: Address,
    /// The address of the veto slasher contract
    pub veto_slasher_address: Address,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ProtocolSettings {
    None,
    #[cfg(feature = "tangle")]
    Tangle(TangleInstanceSettings),
    #[cfg(feature = "eigenlayer")]
    Eigenlayer(EigenlayerContractAddresses),
    #[cfg(feature = "symbiotic")]
    Symbiotic(SymbioticContractAddresses),
}

impl Default for ProtocolSettings {
    fn default() -> Self {
        #[cfg(all(
            feature = "tangle",
            not(feature = "eigenlayer"),
            not(feature = "symbiotic")
        ))]
        {
            Self::Tangle(TangleInstanceSettings::default())
        }
        #[cfg(all(
            feature = "eigenlayer",
            not(feature = "tangle"),
            not(feature = "symbiotic")
        ))]
        {
            Self::Eigenlayer(EigenlayerContractAddresses::default())
        }
        #[cfg(all(
            feature = "symbiotic",
            not(feature = "tangle"),
            not(feature = "eigenlayer")
        ))]
        {
            Self::Symbiotic(SymbioticContractAddresses::default())
        }
        #[cfg(not(any(feature = "tangle", feature = "eigenlayer", feature = "symbiotic")))]
        {
            Self::None
        }
    }
}

impl ProtocolSettings {
    pub fn as_str(&self) -> &'static str {
        match self {
            #[cfg(feature = "tangle")]
            Self::Tangle(_) => "tangle",
            #[cfg(feature = "eigenlayer")]
            Self::Eigenlayer(_) => "eigenlayer",
            #[cfg(feature = "symbiotic")]
            Self::Symbiotic(_) => "symbiotic",
            Self::None => "none",
        }
    }

    #[cfg(feature = "tangle")]
    pub fn tangle(&self) -> Result<&TangleInstanceSettings, &'static str> {
        match self {
            Self::Tangle(settings) => Ok(settings),
            _ => Err("Not a Tangle instance"),
        }
    }

    #[cfg(feature = "eigenlayer")]
    pub fn eigenlayer(&self) -> Result<&EigenlayerContractAddresses, &'static str> {
        match self {
            Self::Eigenlayer(settings) => Ok(settings),
            _ => Err("Not an Eigenlayer instance"),
        }
    }

    #[cfg(feature = "symbiotic")]
    pub fn symbiotic(&self) -> Result<&SymbioticContractAddresses, &'static str> {
        match self {
            Self::Symbiotic(settings) => Ok(settings),
            _ => Err("Not a Symbiotic instance"),
        }
    }

    #[cfg(feature = "tangle")]
    pub fn from_tangle(settings: TangleInstanceSettings) -> Self {
        Self::Tangle(settings)
    }

    #[cfg(feature = "eigenlayer")]
    pub fn from_eigenlayer(settings: EigenlayerContractAddresses) -> Self {
        Self::Eigenlayer(settings)
    }

    #[cfg(feature = "symbiotic")]
    pub fn from_symbiotic(settings: SymbioticContractAddresses) -> Self {
        Self::Symbiotic(settings)
    }
}
