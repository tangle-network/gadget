#![allow(unused_variables, unreachable_code)]

use std::string::{String, ToString};

use std::path::PathBuf;

use crate::BlueprintConfig;
use core::fmt::{Debug, Display};
use core::str::FromStr;
use serde::{Deserialize, Serialize};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets as TanglePriceTargets;
use url::Url;

/// The protocol on which a gadget will be executed.
#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum Protocol {
    #[default]
    Tangle,
}

impl Protocol {
    /// Returns the protocol from the environment variable `PROTOCOL`.
    ///
    /// If the environment variable is not set, it defaults to `Protocol::Tangle`.
    ///
    /// # Errors
    ///
    /// * [`Error::UnsupportedProtocol`] if the protocol is unknown. See [`Protocol`].
    pub fn from_env() -> Result<Self, Error> {
        if let Ok(protocol) = std::env::var("PROTOCOL") {
            return protocol.to_ascii_lowercase().parse::<Protocol>();
        }

        Ok(Protocol::default())
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tangle => "tangle",
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

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct EigenlayerAddresses {
    pub registry_coordinator_address: alloy_primitives::Address,
    pub operator_state_retriever_address: alloy_primitives::Address,
    pub delegation_manager_address: alloy_primitives::Address,
    pub strategy_manager_address: alloy_primitives::Address,
    pub rewards_coordinator_address: alloy_primitives::Address,
    pub avs_directory_address: alloy_primitives::Address,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ProtocolSettings {
    None,
    Tangle(TangleInstanceSettings),
    Eigenlayer(EigenlayerAddresses),
}

impl Default for ProtocolSettings {
    fn default() -> Self {
        Self::None
    }
}

impl ProtocolSettings {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tangle(_) => "tangle",
            Self::Eigenlayer(_) => "eigenlayer",
            Self::None => "none",
        }
    }

    pub fn tangle(&self) -> Result<&TangleInstanceSettings, Error> {
        match self {
            Self::Tangle(settings) => Ok(settings),
            _ => Err(Error::UnexpectedProtocol("Tangle")),
        }
    }

    pub fn eigenlayer(&self) -> Result<&EigenlayerAddresses, Error> {
        match self {
            Self::Eigenlayer(settings) => Ok(settings),
            _ => Err(Error::UnexpectedProtocol("Eigenlayer")),
        }
    }

    pub fn from_tangle(settings: TangleInstanceSettings) -> Self {
        Self::Tangle(settings)
    }

    pub fn from_eigenlayer(settings: EigenlayerAddresses) -> Self {
        Self::Eigenlayer(settings)
    }
}

/// Errors that can occur while loading and using the gadget configuration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Missing `RPC_URL` environment variable.
    #[error("Missing Tangle RPC endpoint")]
    MissingTangleRpcEndpoint,
    /// Missing `KEYSTORE_URI` environment
    #[error("Missing keystore URI")]
    MissingKeystoreUri,
    /// Missing `BLUEPRINT_ID` environment variable
    #[error("Missing blueprint ID")]
    MissingBlueprintId,
    /// Missing `SERVICE_ID` environment variable
    #[error("Missing service ID")]
    MissingServiceId,
    /// Error parsing the blueprint ID.
    #[error(transparent)]
    MalformedBlueprintId(core::num::ParseIntError),
    /// Error parsing the service ID.
    #[error(transparent)]
    MalformedServiceId(core::num::ParseIntError),
    /// Unsupported keystore URI.
    #[error("Unsupported keystore URI: {0}")]
    UnsupportedKeystoreUri(String),
    /// Error parsing the protocol, from the `PROTOCOL` environment variable.
    #[error("Unsupported protocol: {0}")]
    UnsupportedProtocol(String),
    /// Attempting to load the [`ProtocolSettings`] of a protocol differing from the target
    ///
    /// [`ProtocolSettings`]: crate::ProtocolSettings
    #[error("Unexpect protocol, expected {0}")]
    UnexpectedProtocol(&'static str),
    /// No Sr25519 keypair found in the keystore.
    #[error("No Sr25519 keypair found in the keystore")]
    NoSr25519Keypair,
    /// Invalid Sr25519 keypair found in the keystore.
    #[error("Invalid Sr25519 keypair found in the keystore")]
    InvalidSr25519Keypair,
    /// No ECDSA keypair found in the keystore.
    #[error("No ECDSA keypair found in the keystore")]
    NoEcdsaKeypair,
    /// Invalid ECDSA keypair found in the keystore.
    #[error("Invalid ECDSA keypair found in the keystore")]
    InvalidEcdsaKeypair,
    /// Test setup error
    #[error("Test setup error: {0}")]
    TestSetup(String),
    /// Missing `EigenlayerContractAddresses`
    #[error("Missing EigenlayerContractAddresses")]
    MissingEigenlayerContractAddresses,
    /// Missing `SymbioticContractAddresses`
    #[error("Missing SymbioticContractAddresses")]
    MissingSymbioticContractAddresses,
    #[error("Bad RPC Connection: {0}")]
    BadRpcConnection(String),
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Gadget environment.
#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct GadgetConfiguration {
    /// HTTP RPC endpoint for host restaking network (Tangle / Ethereum (Eigenlayer or Symbiotic)).
    pub http_rpc_endpoint: String,
    /// WS RPC endpoint for host restaking network (Tangle / Ethereum (Eigenlayer or Symbiotic)).
    pub ws_rpc_endpoint: String,
    /// The keystore URI for the gadget
    pub keystore_uri: String,
    /// Data directory exclusively for this gadget
    ///
    /// This will be `None` if the blueprint manager was not provided a base directory.
    pub data_dir: Option<PathBuf>,
    /// The type of protocol the gadget is executing on.
    pub protocol: Protocol,
    /// Protocol-specific settings
    pub protocol_settings: ProtocolSettings,
    /// Whether the gadget is in test mode
    pub test_mode: bool,
}

/// Loads the [`GadgetConfiguration`] from the current environment.
/// # Errors
///
/// This function will return an error if any of the required environment variables are missing.
pub fn load(config: ContextConfig) -> Result<GadgetConfiguration, Error> {
    load_inner(config)
}

fn load_inner(config: ContextConfig) -> Result<GadgetConfiguration, Error> {
    tracing::info_span!("gadget");
    let ContextConfig {
        gadget_core_settings:
            GadgetCLICoreSettings::Run {
                test_mode,
                http_rpc_url,
                ws_rpc_url,
                keystore_uri,
                protocol,
                blueprint_id,
                service_id,
                ..
            },
        ..
    } = config;

    let protocol_settings = ProtocolSettings::from_tangle(TangleInstanceSettings {
        blueprint_id: blueprint_id.ok_or(Error::MissingBlueprintId)?,
        service_id: Some(service_id.ok_or(Error::MissingServiceId)?),
    });

    Ok(GadgetConfiguration {
        test_mode,
        http_rpc_endpoint: http_rpc_url.to_string(),
        ws_rpc_endpoint: ws_rpc_url.to_string(),
        keystore_uri,
        data_dir: std::env::var("DATA_DIR").ok().map(PathBuf::from),
        protocol,
        protocol_settings,
    })
}

#[derive(Debug, Default, Clone, clap::Parser, Serialize, Deserialize)]
#[command(name = "General CLI Context")]
pub struct ContextConfig {
    /// Pass through arguments to another command
    #[command(subcommand)]
    pub gadget_core_settings: GadgetCLICoreSettings,
}

#[derive(Debug, Clone, clap::Parser, Serialize, Deserialize)]
pub enum GadgetCLICoreSettings {
    #[command(name = "run")]
    Run {
        #[arg(long, short = 't', env)]
        test_mode: bool,
        #[arg(long, env)]
        #[serde(default = "default_http_rpc_url")]
        http_rpc_url: Url,
        #[arg(long, env)]
        #[serde(default = "default_ws_rpc_url")]
        ws_rpc_url: Url,
        #[arg(long, short = 'd', env)]
        keystore_uri: String,
        #[arg(long, value_enum, env)]
        chain: SupportedChains,
        #[arg(long, short = 'v', global = true, action = clap::ArgAction::Count)]
        verbose: u8,
        /// Whether to use pretty logging
        #[arg(long, env)]
        pretty: bool,
        #[arg(long, env)]
        keystore_password: Option<String>,
        /// The protocol to use
        #[arg(long, value_enum, env)]
        #[serde(default = "default_protocol")]
        protocol: Protocol,
        /// The blueprint ID for Tangle protocol
        #[arg(
            long,
            value_name = "ID",
            env = "BLUEPRINT_ID",
            required_if_eq("protocol", Protocol::Tangle.as_str())
        )]
        blueprint_id: Option<u64>,
        /// The service ID for Tangle protocol
        #[arg(
            long,
            value_name = "ID",
            env = "SERVICE_ID",
            required_if_eq("protocol", Protocol::Tangle.as_str())
        )]
        service_id: Option<u64>,
    },
}

impl Default for GadgetCLICoreSettings {
    fn default() -> Self {
        Self::Run {
            test_mode: false,
            http_rpc_url: default_http_rpc_url(),
            ws_rpc_url: default_ws_rpc_url(),
            keystore_uri: String::new(),
            chain: SupportedChains::default(),
            verbose: 0,
            pretty: false,
            keystore_password: None,
            protocol: Protocol::default(),
            blueprint_id: Some(1),
            service_id: Some(1),
        }
    }
}

impl ContextConfig {
    /// Creates a new context config with the given parameters
    ///
    /// # Arguments
    /// - `http_rpc_url`: The HTTP RPC URL of the target chain
    /// - `ws_rpc_url`: The WebSocket RPC URL of the target chain
    /// - `use_secure_url`: Whether to use a secure URL (ws/wss and http/https)
    /// - `keystore_uri`: The keystore URI as a string
    /// - `chain`: The [`chain`](SupportedChains)
    /// - `protocol`: The [`Protocol`]
    /// - `eigenlayer_contract_addresses`: The [`contract addresses`](EigenlayerContractAddresses) for the necessary EigenLayer contracts
    /// - `symbiotic_contract_addresses`: The [`contract addresses`](SymbioticContractAddresses) for the necessary Symbiotic contracts
    /// - `blueprint_id`: The blueprint ID - only required for Tangle
    /// - `service_id`: The service ID - only required for Tangle
    #[allow(clippy::too_many_arguments)]
    pub fn create_config(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        keystore_password: Option<String>,
        chain: SupportedChains,
        protocol: Protocol,
        protocol_settings: ProtocolSettings,
    ) -> Self {
        // Tangle settings
        let tangle_settings = match protocol_settings {
            ProtocolSettings::Tangle(settings) => Some(settings),
            _ => None,
        };
        let blueprint_id = tangle_settings.map(|s| s.blueprint_id);
        let service_id = tangle_settings.and_then(|s| s.service_id);

        ContextConfig {
            gadget_core_settings: GadgetCLICoreSettings::Run {
                test_mode: false,
                http_rpc_url,
                keystore_uri,
                chain,
                verbose: 3,
                pretty: true,
                keystore_password,
                protocol,
                ws_rpc_url,
                blueprint_id,
                service_id,
            },
        }
    }

    /// Creates a new context config with the given parameters
    ///
    /// # Defaults
    /// - `target_addr`: The same host address as the given http_rpc_url, defaulting to 127.0.0.1 if an error occurs
    /// - `target_port`: The same port as the given http_rpc_url, defaulting to 0 if an error occurs
    /// - `skip_registration`: false
    /// - `keystore_password`: None
    #[allow(clippy::too_many_arguments)]
    pub fn create_config_with_defaults(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        keystore_password: Option<String>,
        chain: SupportedChains,
        protocol: Protocol,
        protocol_settings: ProtocolSettings,
    ) -> Self {
        ContextConfig::create_config(
            http_rpc_url,
            ws_rpc_url,
            keystore_uri,
            keystore_password,
            chain,
            protocol,
            protocol_settings,
        )
    }

    /// Creates a new context config with defaults for Tangle
    pub fn create_tangle_config(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        keystore_password: Option<String>,
        chain: SupportedChains,
        blueprint_id: u64,
        service_id: Option<u64>,
    ) -> Self {
        Self::create_config_with_defaults(
            http_rpc_url,
            ws_rpc_url,
            keystore_uri,
            keystore_password,
            chain,
            Protocol::Tangle,
            ProtocolSettings::Tangle(TangleInstanceSettings {
                blueprint_id,
                service_id,
            }),
        )
    }
}

fn default_protocol() -> Protocol {
    Protocol::Tangle
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

/// Wrapper for `tangle_subxt`'s [`PriceTargets`]
///
/// This provides a [`Default`] impl for a zeroed-out [`PriceTargets`].
///
/// [`PriceTargets`]: tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::PriceTargets
#[derive(Clone)]
pub struct PriceTargets(pub TanglePriceTargets);

impl From<TanglePriceTargets> for PriceTargets {
    fn from(t: TanglePriceTargets) -> Self {
        PriceTargets(t)
    }
}

impl Default for PriceTargets {
    fn default() -> Self {
        Self(TanglePriceTargets {
            cpu: 0,
            mem: 0,
            storage_hdd: 0,
            storage_ssd: 0,
            storage_nvme: 0,
        })
    }
}

#[derive(Clone, Default)]
pub struct TangleConfig {
    pub price_targets: PriceTargets,
    pub exit_after_register: bool,
}

impl TangleConfig {
    pub fn new(price_targets: PriceTargets) -> Self {
        Self {
            price_targets,
            exit_after_register: true,
        }
    }

    pub fn with_exit_after_register(mut self, should_exit_after_registration: bool) -> Self {
        self.exit_after_register = should_exit_after_registration;
        self
    }
}

#[async_trait::async_trait]
impl BlueprintConfig for TangleConfig {
    async fn register(&self, env: &GadgetConfiguration) -> Result<(), crate::Error> {
        todo!()
    }

    async fn requires_registration(&self, env: &GadgetConfiguration) -> Result<bool, crate::Error> {
        Ok(false) // TODO
    }

    fn should_exit_after_registration(&self) -> bool {
        self.exit_after_register
    }
}
