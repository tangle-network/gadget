use super::Error;

use core::fmt::Debug;
use serde::{Deserialize, Serialize};

#[cfg(feature = "eigenlayer")]
use alloy_primitives::address;
#[cfg(any(feature = "eigenlayer", feature = "symbiotic"))]
use alloy_primitives::Address;

/// The protocol on which a gadget will be executed.
#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
        if let Ok(protocol) = gadget_std::env::var("PROTOCOL") {
            return protocol.to_ascii_lowercase().parse::<Protocol>();
        }

        Ok(Protocol::default())
    }

    /// Returns the protocol from the environment variable `PROTOCOL`.
    ///
    /// # Errors
    ///
    /// Infallible
    #[cfg(not(feature = "std"))]
    pub fn from_env() -> Result<Self, Error> {
        Ok(Protocol::default())
    }

    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tangle => "tangle",
            Self::Eigenlayer => "eigenlayer",
            Self::Symbiotic => "symbiotic",
        }
    }
}

impl core::fmt::Display for Protocol {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

#[cfg(feature = "tangle")]
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
            registry_coordinator_address: address!("9E545E3C0baAB3E08CdfD552C960A1050f373042"),
            operator_state_retriever_address: address!("f5059a5D33d5853360D16C683c16e67980206f36"),
            delegation_manager_address: address!("Dc64a140Aa3E981100a9becA4E685f962f0cF6C9"),
            service_manager_address: address!("c3e53F4d16Ae77Db1c982e75a937B9f60FE63690"),
            stake_registry_address: address!("5fc8d32690cc91d4c39d9d3abcbd16989f875707"),
            strategy_manager_address: address!("5FC8d32690cc91D4c39d9d3abcBD16989F875707"),
            avs_directory_address: address!("0165878A594ca255338adfa4d48449f69242Eb8F"),
            rewards_coordinator_address: address!("8A791620dd6260079BF849Dc5567aDC3F2FdC318"),
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
        Self::None
    }
}

impl ProtocolSettings {
    #[must_use]
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

    /// Attempt to extract the [`TangleInstanceSettings`]
    ///
    /// # Errors
    ///
    /// `self` is not [`ProtocolSettings::Tangle`]
    #[cfg(feature = "tangle")]
    #[allow(clippy::match_wildcard_for_single_variants)]
    pub fn tangle(&self) -> Result<&TangleInstanceSettings, Error> {
        match self {
            Self::Tangle(settings) => Ok(settings),
            _ => Err(Error::UnexpectedProtocol("Tangle")),
        }
    }

    /// Attempt to extract the [`EigenlayerContractAddresses`]
    ///
    /// # Errors
    ///
    /// `self` is not [`ProtocolSettings::Eigenlayer`]
    #[cfg(feature = "eigenlayer")]
    #[allow(clippy::match_wildcard_for_single_variants)]
    pub fn eigenlayer(&self) -> Result<&EigenlayerContractAddresses, Error> {
        match self {
            Self::Eigenlayer(settings) => Ok(settings),
            _ => Err(Error::UnexpectedProtocol("Eigenlayer")),
        }
    }

    /// Attempt to extract the [`SymbioticContractAddresses`]
    ///
    /// # Errors
    ///
    /// `self` is not [`ProtocolSettings::Symbiotic`]
    #[cfg(feature = "symbiotic")]
    #[allow(clippy::match_wildcard_for_single_variants)]
    pub fn symbiotic(&self) -> Result<&SymbioticContractAddresses, Error> {
        match self {
            Self::Symbiotic(settings) => Ok(settings),
            _ => Err(Error::UnexpectedProtocol("Symbiotic")),
        }
    }

    #[cfg(feature = "tangle")]
    #[must_use]
    pub fn from_tangle(settings: TangleInstanceSettings) -> Self {
        Self::Tangle(settings)
    }

    #[cfg(feature = "eigenlayer")]
    #[must_use]
    pub fn from_eigenlayer(settings: EigenlayerContractAddresses) -> Self {
        Self::Eigenlayer(settings)
    }

    #[cfg(feature = "symbiotic")]
    #[must_use]
    pub fn from_symbiotic(settings: SymbioticContractAddresses) -> Self {
        Self::Symbiotic(settings)
    }
}
