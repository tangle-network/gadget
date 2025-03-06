#![allow(unused_variables, unreachable_code)]

use std::path::PathBuf;

use crate::error::ConfigError;
use alloc::string::{String, ToString};
use core::fmt::{Debug, Display};
use core::str::FromStr;
#[cfg(feature = "std")]
use gadget_keystore::{Keystore, KeystoreConfig};
#[cfg(feature = "networking")]
pub use libp2p::Multiaddr;
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

    #[cfg(feature = "networking")]
    pub bootnodes: Vec<Multiaddr>,
    /// The port to bind the network to
    #[cfg(feature = "networking")]
    pub network_bind_port: u16,
    #[cfg(feature = "networking")]
    pub enable_mdns: bool,
    /// Whether to enable Kademlia
    #[cfg(feature = "networking")]
    pub enable_kademlia: bool,
    /// The target number of peers to connect to
    #[cfg(feature = "networking")]
    pub target_peer_count: u32,
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

    // TODO: this shouldn't be exclusive to the std feature
    #[cfg(feature = "std")]
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

    #[cfg(feature = "networking")]
    let bootnodes = settings.bootnodes.clone().unwrap_or_default();
    #[cfg(feature = "networking")]
    let network_bind_port = settings.network_bind_port.unwrap_or_default();
    #[cfg(feature = "networking")]
    let enable_mdns = settings.enable_mdns;
    #[cfg(feature = "networking")]
    let enable_kademlia = settings.enable_kademlia;
    #[cfg(feature = "networking")]
    let target_peer_count = settings.target_peer_count.unwrap_or(24);

    let protocol_settings = ProtocolSettings::load(settings)?;

    Ok(BlueprintEnvironment {
        test_mode,
        http_rpc_endpoint: http_rpc_url.to_string(),
        ws_rpc_endpoint: ws_rpc_url.to_string(),
        keystore_uri,
        data_dir: std::env::var("DATA_DIR").ok().map(PathBuf::from),
        protocol_settings,

        #[cfg(feature = "networking")]
        bootnodes,
        #[cfg(feature = "networking")]
        network_bind_port,
        #[cfg(feature = "networking")]
        enable_mdns,
        #[cfg(feature = "networking")]
        enable_kademlia,
        #[cfg(feature = "networking")]
        target_peer_count,
    })
}

#[cfg(feature = "networking")]
impl BlueprintEnvironment {
    /// Start a p2p network with the given `network_config`
    ///
    /// # Errors
    ///
    /// See [`NetworkService::new()`]
    ///
    /// [`NetworkService::new()`]: gadget_networking::NetworkService::new
    #[cfg(feature = "networking")]
    pub fn libp2p_start_network<K: gadget_crypto::KeyType>(
        &self,
        network_config: gadget_networking::NetworkConfig<K>,
        allowed_keys: gadget_networking::service::AllowedKeys<K>,
        allowed_keys_rx: crossbeam_channel::Receiver<gadget_networking::AllowedKeys<K>>,
    ) -> Result<gadget_networking::service_handle::NetworkServiceHandle<K>, crate::error::RunnerError>
    {
        let networking_service =
            gadget_networking::NetworkService::new(network_config, allowed_keys, allowed_keys_rx)?;

        let handle = networking_service.start();

        Ok(handle)
    }

    /// Returns a new `NetworkConfig` for the current environment.
    ///
    /// # Errors
    ///
    /// Missing the following keys in the keystore:
    ///
    /// * `Ed25519`
    /// * `ECDSA`
    #[cfg(feature = "networking")]
    #[allow(clippy::missing_panics_doc)] // Known good Multiaddr
    pub fn libp2p_network_config<K: gadget_crypto::KeyType>(
        &self,
        network_name: impl Into<String>,
        using_evm_address_for_handshake_verification: bool,
    ) -> Result<gadget_networking::NetworkConfig<K>, crate::error::RunnerError> {
        use gadget_keystore::backends::Backend;
        use gadget_keystore::crypto::sp_core::SpEd25519 as LibP2PKeyType;

        let keystore_config = gadget_keystore::KeystoreConfig::new().fs_root(&self.keystore_uri);
        let keystore = gadget_keystore::Keystore::new(keystore_config)?;
        let ed25519_pub_key = keystore.first_local::<LibP2PKeyType>()?;
        let ed25519_pair = keystore.get_secret::<LibP2PKeyType>(&ed25519_pub_key)?;
        let network_identity = libp2p::identity::Keypair::ed25519_from_bytes(ed25519_pair.seed())
            .expect("should be valid");

        let ecdsa_pub_key = keystore.first_local::<K>()?;
        let ecdsa_pair = keystore.get_secret::<K>(&ecdsa_pub_key)?;

        let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", self.network_bind_port)
            .parse()
            .expect("valid multiaddr; qed");

        let network_name: String = network_name.into();
        let network_config = gadget_networking::NetworkConfig {
            instance_id: network_name.clone(),
            network_name,
            instance_key_pair: ecdsa_pair,
            local_key: network_identity,
            listen_addr,
            target_peer_count: self.target_peer_count,
            bootstrap_peers: self.bootnodes.clone(),
            enable_mdns: self.enable_mdns,
            enable_kademlia: self.enable_kademlia,
            using_evm_address_for_handshake_verification,
        };

        Ok(network_config)
    }
}

#[derive(Debug, Default, Clone, clap::Parser, Serialize, Deserialize)]
#[command(name = "General CLI Context")]
pub struct ContextConfig {
    /// Pass through arguments to another command
    #[command(subcommand)]
    pub blueprint_core_settings: BlueprintCliCoreSettings,
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
    #[allow(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        clippy::match_wildcard_for_single_variants
    )]
    #[must_use]
    pub fn create_config(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        keystore_password: Option<String>,
        chain: SupportedChains,
        protocol: Protocol,
        protocol_settings: ProtocolSettings,
    ) -> Self {
        // Eigenlayer addresses
        #[cfg(feature = "eigenlayer")]
        let eigenlayer_settings = match protocol_settings {
            ProtocolSettings::Eigenlayer(settings) => Some(settings),
            _ => None,
        };
        #[cfg(feature = "eigenlayer")]
        let allocation_manager = eigenlayer_settings.map(|s| s.allocation_manager_address);
        #[cfg(feature = "eigenlayer")]
        let registry_coordinator = eigenlayer_settings.map(|s| s.registry_coordinator_address);
        #[cfg(feature = "eigenlayer")]
        let operator_state_retriever =
            eigenlayer_settings.map(|s| s.operator_state_retriever_address);
        #[cfg(feature = "eigenlayer")]
        let delegation_manager = eigenlayer_settings.map(|s| s.delegation_manager_address);
        #[cfg(feature = "eigenlayer")]
        let service_manager = eigenlayer_settings.map(|s| s.service_manager_address);
        #[cfg(feature = "eigenlayer")]
        let stake_registry = eigenlayer_settings.map(|s| s.stake_registry_address);
        #[cfg(feature = "eigenlayer")]
        let strategy_manager = eigenlayer_settings.map(|s| s.strategy_manager_address);
        #[cfg(feature = "eigenlayer")]
        let avs_directory = eigenlayer_settings.map(|s| s.avs_directory_address);
        #[cfg(feature = "eigenlayer")]
        let rewards_coordinator = eigenlayer_settings.map(|s| s.rewards_coordinator_address);
        #[cfg(feature = "eigenlayer")]
        let permission_controller = eigenlayer_settings.map(|s| s.permission_controller_address);

        // Tangle settings
        #[cfg(feature = "tangle")]
        let tangle_settings = match protocol_settings {
            ProtocolSettings::Tangle(settings) => Some(settings),
            _ => None,
        };
        #[cfg(feature = "tangle")]
        let blueprint_id = tangle_settings.map(|s| s.blueprint_id);
        #[cfg(feature = "tangle")]
        let service_id = tangle_settings.and_then(|s| s.service_id);

        #[cfg(feature = "networking")]
        let enable_mdns = cfg!(debug_assertions);
        #[cfg(feature = "networking")]
        let enable_kademlia = !cfg!(debug_assertions);

        ContextConfig {
            blueprint_core_settings: BlueprintCliCoreSettings::Run(BlueprintSettings {
                test_mode: false,
                http_rpc_url,
                #[cfg(feature = "networking")]
                bootnodes: None,
                #[cfg(feature = "networking")]
                network_bind_port: None,
                #[cfg(feature = "networking")]
                enable_mdns,
                #[cfg(feature = "networking")]
                enable_kademlia,
                #[cfg(feature = "networking")]
                target_peer_count: None,
                keystore_uri,
                chain,
                verbose: 3,
                pretty: true,
                keystore_password,
                protocol: Some(protocol),
                ws_rpc_url,
                #[cfg(feature = "tangle")]
                blueprint_id,
                #[cfg(feature = "tangle")]
                service_id,
                #[cfg(feature = "eigenlayer")]
                allocation_manager,
                #[cfg(feature = "eigenlayer")]
                registry_coordinator,
                #[cfg(feature = "eigenlayer")]
                operator_state_retriever,
                #[cfg(feature = "eigenlayer")]
                delegation_manager,
                #[cfg(feature = "eigenlayer")]
                stake_registry,
                #[cfg(feature = "eigenlayer")]
                service_manager,
                #[cfg(feature = "eigenlayer")]
                strategy_manager,
                #[cfg(feature = "eigenlayer")]
                avs_directory,
                #[cfg(feature = "eigenlayer")]
                rewards_coordinator,
                #[cfg(feature = "eigenlayer")]
                permission_controller,
            }),
        }
    }

    /// Creates a new context config with the given parameters
    ///
    /// # Defaults
    /// - `target_addr`: The same host address as the given `http_rpc_url`, defaulting to 127.0.0.1 if an error occurs
    /// - `target_port`: The same port as the given `http_rpc_url`, defaulting to 0 if an error occurs
    /// - `skip_registration`: false
    /// - `keystore_password`: None
    #[allow(clippy::too_many_arguments)]
    #[must_use]
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

    /// Creates a new context config with defaults for Eigenlayer
    #[cfg(feature = "eigenlayer")]
    #[must_use]
    pub fn create_eigenlayer_config(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        keystore_password: Option<String>,
        chain: SupportedChains,
        eigenlayer_contract_addresses: crate::eigenlayer::config::EigenlayerProtocolSettings,
    ) -> Self {
        Self::create_config_with_defaults(
            http_rpc_url,
            ws_rpc_url,
            keystore_uri,
            keystore_password,
            chain,
            Protocol::Eigenlayer,
            ProtocolSettings::Eigenlayer(eigenlayer_contract_addresses),
        )
    }

    /// Creates a new context config with defaults for Tangle
    #[cfg(feature = "tangle")]
    #[must_use]
    pub fn create_tangle_config(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        keystore_password: Option<String>,
        chain: SupportedChains,
        blueprint_id: u64,
        service_id: Option<u64>,
    ) -> Self {
        use crate::tangle::config::TangleProtocolSettings;

        Self::create_config_with_defaults(
            http_rpc_url,
            ws_rpc_url,
            keystore_uri,
            keystore_password,
            chain,
            Protocol::Tangle,
            ProtocolSettings::Tangle(TangleProtocolSettings {
                blueprint_id,
                service_id,
            }),
        )
    }
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

    // ========
    // NETWORKING
    // ========
    #[cfg(feature = "networking")]
    #[arg(long, value_parser = <Multiaddr as gadget_std::str::FromStr>::from_str, action = clap::ArgAction::Append, env)]
    #[serde(default)]
    bootnodes: Option<Vec<Multiaddr>>,
    #[cfg(feature = "networking")]
    #[arg(long, env)]
    #[serde(default)]
    network_bind_port: Option<u16>,
    #[cfg(feature = "networking")]
    #[arg(long, env)]
    #[serde(default)]
    enable_mdns: bool,
    #[cfg(feature = "networking")]
    #[arg(long, env)]
    #[serde(default)]
    enable_kademlia: bool,
    #[cfg(feature = "networking")]
    #[arg(long, env)]
    #[serde(default)]
    target_peer_count: Option<u32>,

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

            // Networking
            #[cfg(feature = "networking")]
            bootnodes: None,
            #[cfg(feature = "networking")]
            network_bind_port: None,
            #[cfg(feature = "networking")]
            enable_mdns: false,
            #[cfg(feature = "networking")]
            enable_kademlia: false,
            #[cfg(feature = "networking")]
            target_peer_count: None,

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

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
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
