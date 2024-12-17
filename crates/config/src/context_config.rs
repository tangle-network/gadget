use super::*;
use gadget_std::fmt::Debug;
use gadget_std::str::FromStr;
use serde::{Deserialize, Serialize};
use supported_chains::SupportedChains;
use url::Url;

#[cfg(feature = "networking")]
use libp2p::Multiaddr;

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
        #[cfg(feature = "networking")]
        #[arg(long, value_parser = <Multiaddr as gadget_std::str::FromStr>::from_str, action = clap::ArgAction::Append, env)]
        #[serde(default)]
        bootnodes: Option<Vec<Multiaddr>>,
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
        #[cfg(feature = "tangle")]
        #[arg(
            long,
            value_name = "ID",
            env = "BLUEPRINT_ID",
            required_if_eq("protocol", Protocol::Tangle.as_str())
        )]
        blueprint_id: Option<u64>,
        /// The service ID for Tangle protocol
        #[cfg(feature = "tangle")]
        #[arg(
            long,
            value_name = "ID",
            env = "SERVICE_ID",
            required_if_eq("protocol", Protocol::Tangle.as_str())
        )]
        service_id: Option<u64>,
        /// The address of the registry coordinator
        #[cfg(feature = "eigenlayer")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "REGISTRY_COORDINATOR_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str()),
        )]
        registry_coordinator: Option<alloy_primitives::Address>,
        /// The address of the operator state retriever
        #[cfg(feature = "eigenlayer")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "OPERATOR_STATE_RETRIEVER_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        operator_state_retriever: Option<alloy_primitives::Address>,
        /// The address of the delegation manager
        #[cfg(feature = "eigenlayer")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "DELEGATION_MANAGER_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        delegation_manager: Option<alloy_primitives::Address>,
        /// The address of the strategy manager
        #[cfg(feature = "eigenlayer")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "STRATEGY_MANAGER_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        strategy_manager: Option<alloy_primitives::Address>,
        /// The address of the Service Manager
        #[cfg(feature = "eigenlayer")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "SERVICE_MANAGER_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        service_manager: Option<alloy_primitives::Address>,
        /// The address of the Stake Registry
        #[cfg(feature = "eigenlayer")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "STAKE_REGISTRY_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        stake_registry: Option<alloy_primitives::Address>,
        /// The address of the AVS directory
        #[cfg(feature = "eigenlayer")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "AVS_DIRECTORY_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        avs_directory: Option<alloy_primitives::Address>,
        /// The address of the rewards coordinator
        #[cfg(feature = "eigenlayer")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "REWARDS_COORDINATOR_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        rewards_coordinator: Option<alloy_primitives::Address>,
        /// The address of the operator registry
        #[cfg(feature = "symbiotic")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "OPERATOR_REGISTRY_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        operator_registry: Option<alloy_primitives::Address>,
        /// The address of the network registry
        #[cfg(feature = "symbiotic")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "NETWORK_REGISTRY_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        network_registry: Option<alloy_primitives::Address>,
        /// The address of the base delegator
        #[cfg(feature = "symbiotic")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "BASE_DELEGATOR_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        base_delegator: Option<alloy_primitives::Address>,
        /// The address of the network opt-in service
        #[cfg(feature = "symbiotic")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "NETWORK_OPT_IN_SERVICE_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        network_opt_in_service: Option<alloy_primitives::Address>,
        /// The address of the vault opt-in service
        #[cfg(feature = "symbiotic")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "VAULT_OPT_IN_SERVICE_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        vault_opt_in_service: Option<alloy_primitives::Address>,
        /// The address of the slasher
        #[cfg(feature = "symbiotic")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "SLASHER_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        slasher: Option<alloy_primitives::Address>,
        /// The address of the veto slasher
        #[cfg(feature = "symbiotic")]
        #[arg(
            long,
            value_name = "ADDR",
            env = "VETO_SLASHER_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        veto_slasher: Option<alloy_primitives::Address>,
    },
}

impl Default for GadgetCLICoreSettings {
    fn default() -> Self {
        Self::Run {
            test_mode: false,
            http_rpc_url: default_http_rpc_url(),
            ws_rpc_url: default_ws_rpc_url(),
            #[cfg(feature = "networking")]
            bootnodes: None,
            keystore_uri: String::new(),
            chain: SupportedChains::default(),
            verbose: 0,
            pretty: false,
            keystore_password: None,
            protocol: Protocol::default(),
            #[cfg(feature = "tangle")]
            blueprint_id: Some(1),
            #[cfg(feature = "tangle")]
            service_id: Some(1),
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
            #[cfg(feature = "symbiotic")]
            operator_registry: None,
            #[cfg(feature = "symbiotic")]
            network_registry: None,
            #[cfg(feature = "symbiotic")]
            base_delegator: None,
            #[cfg(feature = "symbiotic")]
            network_opt_in_service: None,
            #[cfg(feature = "symbiotic")]
            vault_opt_in_service: None,
            #[cfg(feature = "symbiotic")]
            slasher: None,
            #[cfg(feature = "symbiotic")]
            veto_slasher: None,
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
        // Eigenlayer addresses
        #[cfg(feature = "eigenlayer")]
        let eigenlayer_settings = match protocol_settings {
            ProtocolSettings::Eigenlayer(settings) => Some(settings),
            _ => None,
        };
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

        // Symbiotic addresses
        #[cfg(feature = "symbiotic")]
        let symbiotic_settings = match protocol_settings {
            ProtocolSettings::Symbiotic(settings) => Some(settings),
            _ => None,
        };
        #[cfg(feature = "symbiotic")]
        let operator_registry = symbiotic_settings.map(|s| s.operator_registry_address);
        #[cfg(feature = "symbiotic")]
        let network_registry = symbiotic_settings.map(|s| s.network_registry_address);
        #[cfg(feature = "symbiotic")]
        let base_delegator = symbiotic_settings.map(|s| s.base_delegator_address);
        #[cfg(feature = "symbiotic")]
        let network_opt_in_service = symbiotic_settings.map(|s| s.network_opt_in_service_address);
        #[cfg(feature = "symbiotic")]
        let vault_opt_in_service = symbiotic_settings.map(|s| s.vault_opt_in_service_address);
        #[cfg(feature = "symbiotic")]
        let slasher = symbiotic_settings.map(|s| s.slasher_address);
        #[cfg(feature = "symbiotic")]
        let veto_slasher = symbiotic_settings.map(|s| s.veto_slasher_address);

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

        ContextConfig {
            gadget_core_settings: GadgetCLICoreSettings::Run {
                test_mode: false,
                http_rpc_url,
                #[cfg(feature = "networking")]
                bootnodes: None,
                keystore_uri,
                chain,
                verbose: 3,
                pretty: true,
                keystore_password,
                protocol,
                ws_rpc_url,
                #[cfg(feature = "tangle")]
                blueprint_id,
                #[cfg(feature = "tangle")]
                service_id,
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
                #[cfg(feature = "symbiotic")]
                operator_registry,
                #[cfg(feature = "symbiotic")]
                network_registry,
                #[cfg(feature = "symbiotic")]
                base_delegator,
                #[cfg(feature = "symbiotic")]
                network_opt_in_service,
                #[cfg(feature = "symbiotic")]
                vault_opt_in_service,
                #[cfg(feature = "symbiotic")]
                slasher,
                #[cfg(feature = "symbiotic")]
                veto_slasher,
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

    /// Creates a new context config with defaults for Eigenlayer
    #[cfg(feature = "eigenlayer")]
    pub fn create_eigenlayer_config(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        keystore_password: Option<String>,
        chain: SupportedChains,
        eigenlayer_contract_addresses: crate::protocol::EigenlayerContractAddresses,
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

    /// Creates a new context config with defaults for Symbiotic
    #[cfg(feature = "symbiotic")]
    pub fn create_symbiotic_config(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        keystore_password: Option<String>,
        chain: SupportedChains,
        symbiotic_contract_addresses: crate::protocol::SymbioticContractAddresses,
    ) -> Self {
        Self::create_config_with_defaults(
            http_rpc_url,
            ws_rpc_url,
            keystore_uri,
            keystore_password,
            chain,
            Protocol::Symbiotic,
            ProtocolSettings::Symbiotic(symbiotic_contract_addresses),
        )
    }

    /// Creates a new context config with defaults for Tangle
    #[cfg(feature = "tangle")]
    pub fn create_tangle_config(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        keystore_password: Option<String>,
        chain: SupportedChains,
        blueprint_id: u64,
        service_id: Option<u64>,
    ) -> Self {
        use protocol::TangleInstanceSettings;

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
