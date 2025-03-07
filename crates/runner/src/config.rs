#![allow(unused_variables, unreachable_code)]

use std::path::PathBuf;

use crate::error::ConfigError;
use alloc::string::{String, ToString};
use core::fmt::{Debug, Display};
use core::str::FromStr;
use gadget_keystore::{Keystore, KeystoreConfig};
use serde::{Deserialize, Serialize};
use url::Url;

pub trait ProtocolSettingsT: Sized + 'static {
    type Settings;

    fn load(settings: BlueprintSettings)
    -> Result<Self, Box<dyn core::error::Error + Send + Sync>>;
    fn protocol(&self) -> &'static str;
    fn settings(&self) -> &Self::Settings;
}

/// The protocol on which a blueprint will be executed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(
    feature = "std",
    derive(clap::ValueEnum),
    clap(rename_all = "lowercase")
)]
#[non_exhaustive]
pub enum Protocol {
    #[cfg(feature = "tangle")]
    Tangle,
    #[cfg(feature = "eigenlayer")]
    Eigenlayer,
    #[cfg(feature = "symbiotic")]
    Symbiotic,
}

impl Protocol {
    /// Returns the protocol from the environment variable `PROTOCOL`.
    ///
    /// If the environment variable is not set, it will return `None`.
    ///
    /// # Errors
    ///
    /// * [`Error::UnsupportedProtocol`] if the protocol is unknown. See [`Protocol`].
    pub fn from_env() -> Result<Option<Self>, ConfigError> {
        if let Ok(protocol) = std::env::var("PROTOCOL") {
            return protocol.to_ascii_lowercase().parse::<Protocol>().map(Some);
        }

        Ok(None)
    }

    pub fn as_str(&self) -> &'static str {
        #[allow(unreachable_patterns)]
        match self {
            #[cfg(feature = "tangle")]
            Self::Tangle => "tangle",
            #[cfg(feature = "eigenlayer")]
            Self::Eigenlayer => "eigenlayer",
            #[cfg(feature = "symbiotic")]
            Self::Symbiotic => "symbiotic",
            _ => unreachable!("should be exhaustive"),
        }
    }
}

impl core::fmt::Display for Protocol {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl core::str::FromStr for Protocol {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            #[cfg(feature = "tangle")]
            "tangle" => Ok(Self::Tangle),
            #[cfg(feature = "eigenlayer")]
            "eigenlayer" => Ok(Self::Eigenlayer),
            #[cfg(feature = "symbiotic")]
            "symbiotic" => Ok(Self::Symbiotic),
            _ => Err(ConfigError::UnsupportedProtocol(s.to_string())),
        }
    }
}

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ProtocolSettings {
    #[default]
    None,
    #[cfg(feature = "tangle")]
    Tangle(crate::tangle::config::TangleProtocolSettings),
    #[cfg(feature = "eigenlayer")]
    Eigenlayer(crate::eigenlayer::config::EigenlayerProtocolSettings),
    #[cfg(feature = "symbiotic")]
    Symbiotic,
}

impl ProtocolSettingsT for ProtocolSettings {
    type Settings = Self;

    fn load(
        settings: BlueprintSettings,
    ) -> Result<Self, Box<dyn core::error::Error + Send + Sync>> {
        #[allow(unreachable_patterns)]
        let protocol_settings = match settings.protocol {
            #[cfg(feature = "tangle")]
            Some(Protocol::Tangle) => {
                use crate::tangle::config::TangleProtocolSettings;
                let settings = TangleProtocolSettings::load(settings)?;
                ProtocolSettings::Tangle(settings)
            }
            #[cfg(feature = "eigenlayer")]
            Some(Protocol::Eigenlayer) => {
                use crate::eigenlayer::config::EigenlayerProtocolSettings;
                let settings = EigenlayerProtocolSettings::load(settings)?;
                ProtocolSettings::Eigenlayer(settings)
            }
            #[cfg(feature = "symbiotic")]
            Some(Protocol::Symbiotic) => {
                todo!()
            }
            None => ProtocolSettings::None,
            _ => unreachable!("should be exhaustive"),
        };

        Ok(protocol_settings)
    }

    fn protocol(&self) -> &'static str {
        match self {
            #[cfg(feature = "tangle")]
            ProtocolSettings::Tangle(val) => val.protocol(),
            #[cfg(feature = "eigenlayer")]
            ProtocolSettings::Eigenlayer(val) => val.protocol(),
            #[cfg(feature = "symbiotic")]
            ProtocolSettings::Symbiotic => "symbiotic",
            _ => unreachable!("should be exhaustive"),
        }
    }

    fn settings(&self) -> &Self::Settings {
        self
    }
}

impl ProtocolSettings {
    /// Attempt to extract the [`TangleInstanceSettings`]
    ///
    /// # Errors
    ///
    /// `self` is not [`ProtocolSettings::Tangle`]
    #[cfg(feature = "tangle")]
    #[allow(clippy::match_wildcard_for_single_variants)]
    pub fn tangle(&self) -> Result<&crate::tangle::config::TangleProtocolSettings, ConfigError> {
        match self {
            Self::Tangle(settings) => Ok(settings),
            _ => Err(ConfigError::UnexpectedProtocol("Tangle")),
        }
    }

    /// Attempt to extract the [`EigenlayerProtocolSettings`]
    ///
    /// # Errors
    ///
    /// `self` is not [`ProtocolSettings::Eigenlayer`]
    #[cfg(feature = "eigenlayer")]
    #[allow(clippy::match_wildcard_for_single_variants)]
    pub fn eigenlayer(
        &self,
    ) -> Result<&crate::eigenlayer::config::EigenlayerProtocolSettings, ConfigError> {
        match self {
            Self::Eigenlayer(settings) => Ok(settings),
            _ => Err(ConfigError::UnexpectedProtocol("Eigenlayer")),
        }
    }

    /// Attempt to extract the [`SymbioticContractAddresses`]
    ///
    /// # Errors
    ///
    /// `self` is not [`ProtocolSettings::Symbiotic`]
    #[cfg(feature = "symbiotic")]
    #[allow(clippy::match_wildcard_for_single_variants)]
    pub fn symbiotic(&self) -> Result<(), ConfigError> {
        todo!()
    }
}

/// Description of the environment in which the blueprint is running
#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct BlueprintEnvironment {
    /// HTTP RPC endpoint for host restaking network (Tangle / Ethereum (Eigenlayer or Symbiotic)).
    pub http_rpc_endpoint: String,
    /// WS RPC endpoint for host restaking network (Tangle / Ethereum (Eigenlayer or Symbiotic)).
    pub ws_rpc_endpoint: String,
    /// The keystore URI for the blueprint
    pub keystore_uri: String,
    /// Data directory exclusively for this blueprint
    ///
    /// This will be `None` if the blueprint manager was not provided a base directory.
    pub data_dir: Option<PathBuf>,
    /// Protocol-specific settings
    pub protocol_settings: ProtocolSettings,
    /// Whether the blueprint is in test mode
    pub test_mode: bool,
}

impl BlueprintEnvironment {
    /// Loads the [`BlueprintEnvironment`] from the current environment.
    ///
    /// # Errors
    ///
    /// This function will return an error if any of the required environment variables are missing.
    pub fn load(config: ContextConfig) -> Result<BlueprintEnvironment, ConfigError> {
        load_inner(config)
    }

    pub fn keystore(&self) -> Keystore {
        let config = KeystoreConfig::new().fs_root(self.keystore_uri.clone());
        Keystore::new(config).expect("Failed to create keystore")
    }
}

fn load_inner(config: ContextConfig) -> Result<BlueprintEnvironment, ConfigError> {
    let ContextConfig {
        blueprint_core_settings: BlueprintCliCoreSettings::Run(settings),
        ..
    } = config;

    let test_mode = settings.test_mode;
    let http_rpc_url = settings.http_rpc_url.clone();
    let ws_rpc_url = settings.ws_rpc_url.clone();
    let keystore_uri = settings.keystore_uri.clone();

    let protocol_settings = ProtocolSettings::load(settings)?;

    Ok(BlueprintEnvironment {
        test_mode,
        http_rpc_endpoint: http_rpc_url.to_string(),
        ws_rpc_endpoint: ws_rpc_url.to_string(),
        keystore_uri,
        data_dir: std::env::var("DATA_DIR").ok().map(PathBuf::from),
        protocol_settings,
    })
}

#[derive(Debug, Default, Clone, clap::Parser, Serialize, Deserialize)]
#[command(name = "General CLI Context")]
pub struct ContextConfig {
    /// Pass through arguments to another command
    #[command(subcommand)]
    pub blueprint_core_settings: BlueprintCliCoreSettings,
}

#[derive(Debug, Clone, clap::Parser, Serialize, Deserialize)]
pub enum BlueprintCliCoreSettings {
    #[command(name = "run")]
    Run(BlueprintSettings),
}

impl Default for BlueprintCliCoreSettings {
    fn default() -> Self {
        BlueprintCliCoreSettings::Run(BlueprintSettings::default())
    }
}

#[derive(Debug, Clone, clap::Parser, Serialize, Deserialize)]
pub struct BlueprintSettings {
    #[arg(long, short = 't', env)]
    pub test_mode: bool,
    #[arg(long, env)]
    #[serde(default = "default_http_rpc_url")]
    pub http_rpc_url: Url,
    #[arg(long, env)]
    #[serde(default = "default_ws_rpc_url")]
    pub ws_rpc_url: Url,
    #[arg(long, short = 'd', env)]
    pub keystore_uri: String,
    #[arg(long, value_enum, env)]
    pub chain: SupportedChains,
    #[arg(long, short = 'v', global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Whether to use pretty logging
    #[arg(long, env)]
    pub pretty: bool,
    #[arg(long, env)]
    pub keystore_password: Option<String>,
    /// The protocol to use
    #[arg(long, value_enum, env)]
    pub protocol: Option<Protocol>,

    // =======
    // TANGLE
    // =======
    #[cfg(feature = "tangle")]
    /// The blueprint ID for Tangle protocol
    #[arg(
        long,
        value_name = "ID",
        env = "BLUEPRINT_ID",
        required_if_eq("protocol", Protocol::Tangle.as_str())
    )]
    pub blueprint_id: Option<u64>,
    #[cfg(feature = "tangle")]
    /// The service ID for Tangle protocol
    #[arg(
        long,
        value_name = "ID",
        env = "SERVICE_ID",
        required_if_eq("protocol", Protocol::Tangle.as_str())
    )]
    pub service_id: Option<u64>,

    // ========
    // EIGENLAYER
    // ========
    /// The address of the allocation manager
    #[cfg(feature = "eigenlayer")]
    #[arg(
        long,
        value_name = "ADDR",
        env = "ALLOCATION_MANAGER_ADDRESS",
        required_if_eq("protocol", Protocol::Eigenlayer.as_str()),
    )]
    pub allocation_manager: Option<alloy_primitives::Address>,
    #[cfg(feature = "eigenlayer")]
    /// The address of the registry coordinator
    #[arg(
        long,
        value_name = "ADDR",
        env = "REGISTRY_COORDINATOR_ADDRESS",
        required_if_eq("protocol", Protocol::Eigenlayer.as_str()),
    )]
    pub registry_coordinator: Option<alloy_primitives::Address>,
    #[cfg(feature = "eigenlayer")]
    /// The address of the operator state retriever
    #[arg(
        long,
        value_name = "ADDR",
        env = "OPERATOR_STATE_RETRIEVER_ADDRESS",
        required_if_eq("protocol", Protocol::Eigenlayer.as_str())
    )]
    pub operator_state_retriever: Option<alloy_primitives::Address>,
    #[cfg(feature = "eigenlayer")]
    /// The address of the delegation manager
    #[arg(
        long,
        value_name = "ADDR",
        env = "DELEGATION_MANAGER_ADDRESS",
        required_if_eq("protocol", Protocol::Eigenlayer.as_str())
    )]
    pub delegation_manager: Option<alloy_primitives::Address>,
    #[cfg(feature = "eigenlayer")]
    /// The address of the strategy manager
    #[arg(
        long,
        value_name = "ADDR",
        env = "STRATEGY_MANAGER_ADDRESS",
        required_if_eq("protocol", Protocol::Eigenlayer.as_str())
    )]
    pub strategy_manager: Option<alloy_primitives::Address>,
    #[cfg(feature = "eigenlayer")]
    /// The address of the Service Manager
    #[arg(
        long,
        value_name = "ADDR",
        env = "SERVICE_MANAGER_ADDRESS",
        required_if_eq("protocol", Protocol::Eigenlayer.as_str())
    )]
    pub service_manager: Option<alloy_primitives::Address>,
    #[cfg(feature = "eigenlayer")]
    /// The address of the Stake Registry
    #[arg(
        long,
        value_name = "ADDR",
        env = "STAKE_REGISTRY_ADDRESS",
        required_if_eq("protocol", Protocol::Eigenlayer.as_str())
    )]
    pub stake_registry: Option<alloy_primitives::Address>,
    #[cfg(feature = "eigenlayer")]
    /// The address of the AVS directory
    #[arg(
        long,
        value_name = "ADDR",
        env = "AVS_DIRECTORY_ADDRESS",
        required_if_eq("protocol", Protocol::Eigenlayer.as_str())
    )]
    pub avs_directory: Option<alloy_primitives::Address>,
    #[cfg(feature = "eigenlayer")]
    /// The address of the rewards coordinator
    #[arg(
        long,
        value_name = "ADDR",
        env = "REWARDS_COORDINATOR_ADDRESS",
        required_if_eq("protocol", Protocol::Eigenlayer.as_str())
    )]
    pub rewards_coordinator: Option<alloy_primitives::Address>,
    /// The address of the permission controller
    #[cfg(feature = "eigenlayer")]
    #[arg(
        long,
        value_name = "ADDR",
        env = "PERMISSION_CONTROLLER_ADDRESS",
        required_if_eq("protocol", Protocol::Eigenlayer.as_str()),
    )]
    pub permission_controller: Option<alloy_primitives::Address>,
}

impl Default for BlueprintSettings {
    fn default() -> Self {
        Self {
            test_mode: false,
            http_rpc_url: default_http_rpc_url(),
            ws_rpc_url: default_ws_rpc_url(),
            keystore_uri: String::new(),
            chain: SupportedChains::default(),
            verbose: 0,
            pretty: false,
            keystore_password: None,
            protocol: None,

            // =======
            // TANGLE
            // =======
            #[cfg(feature = "tangle")]
            blueprint_id: None,
            #[cfg(feature = "tangle")]
            service_id: None,

            // ========
            // EIGENLAYER
            // ========
            #[cfg(feature = "eigenlayer")]
            allocation_manager: None,
            #[cfg(feature = "eigenlayer")]
            registry_coordinator: None,
            #[cfg(feature = "eigenlayer")]
            operator_state_retriever: None,
            #[cfg(feature = "eigenlayer")]
            delegation_manager: None,
            #[cfg(feature = "eigenlayer")]
            service_manager: None,
            #[cfg(feature = "eigenlayer")]
            stake_registry: None,
            #[cfg(feature = "eigenlayer")]
            strategy_manager: None,
            #[cfg(feature = "eigenlayer")]
            avs_directory: None,
            #[cfg(feature = "eigenlayer")]
            rewards_coordinator: None,
            #[cfg(feature = "eigenlayer")]
            permission_controller: None,
        }
    }
}

fn default_http_rpc_url() -> Url {
    Url::from_str("http://127.0.0.1:9944").unwrap()
}

fn default_ws_rpc_url() -> Url {
    Url::from_str("ws://127.0.0.1:9944").unwrap()
}

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize, clap::ValueEnum)]
#[clap(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SupportedChains {
    #[default]
    LocalTestnet,
    LocalMainnet,
    Testnet,
    Mainnet,
}

impl FromStr for SupportedChains {
    type Err = String;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        match s {
            "local_testnet" => Ok(SupportedChains::LocalTestnet),
            "local_mainnet" => Ok(SupportedChains::LocalMainnet),
            "testnet" => Ok(SupportedChains::Testnet),
            "mainnet" => Ok(SupportedChains::Mainnet),
            _ => Err(format!("Invalid chain: {}", s)),
        }
    }
}

impl Display for SupportedChains {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SupportedChains::LocalTestnet => write!(f, "local_testnet"),
            SupportedChains::LocalMainnet => write!(f, "local_mainnet"),
            SupportedChains::Testnet => write!(f, "testnet"),
            SupportedChains::Mainnet => write!(f, "mainnet"),
        }
    }
}
