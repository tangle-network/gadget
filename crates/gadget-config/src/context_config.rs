use super::*;
use crate::config::protocol::{EigenlayerContractAddresses, SymbioticContractAddresses};
use alloc::string::String;
use alloy_primitives::Address;
use gadget_std::fmt::Debug;
use gadget_std::net::IpAddr;
use gadget_std::str::FromStr;
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use url::Url;

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
        #[arg(long, short = 'b', env)]
        target_addr: IpAddr,
        #[arg(long, short = 'p', env)]
        target_port: u16,
        #[arg(long, short = 's', env)]
        use_secure_url: bool,
        #[arg(long, short = 't', env)]
        test_mode: bool,
        #[arg(long, short = 'l', env)]
        log_id: Option<String>,
        #[arg(long, env)]
        #[serde(default = "gadget_io::defaults::http_rpc_url")]
        http_rpc_url: Url,
        #[arg(long, env)]
        #[serde(default = "gadget_io::defaults::ws_rpc_url")]
        ws_rpc_url: Url,
        #[arg(long, value_parser = <Multiaddr as std::str::FromStr>::from_str, action = clap::ArgAction::Append, env)]
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
        /// Whether to skip the registration process for a Blueprint
        #[arg(long, env = "SKIP_REGISTRATION")]
        skip_registration: bool,
        /// The address of the registry coordinator
        #[arg(
            long,
            value_name = "ADDR",
            env = "REGISTRY_COORDINATOR_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str()),
        )]
        registry_coordinator: Option<Address>,
        /// The address of the operator state retriever
        #[arg(
            long,
            value_name = "ADDR",
            env = "OPERATOR_STATE_RETRIEVER_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        operator_state_retriever: Option<Address>,
        /// The address of the delegation manager
        #[arg(
            long,
            value_name = "ADDR",
            env = "DELEGATION_MANAGER_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        delegation_manager: Option<Address>,
        /// The address of the strategy manager
        #[arg(
            long,
            value_name = "ADDR",
            env = "STRATEGY_MANAGER_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        strategy_manager: Option<Address>,
        /// The address of the Service Manager
        #[arg(
            long,
            value_name = "ADDR",
            env = "SERVICE_MANAGER_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        service_manager: Option<Address>,
        /// The address of the Stake Registry
        #[arg(
            long,
            value_name = "ADDR",
            env = "STAKE_REGISTRY_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        stake_registry: Option<Address>,
        /// The address of the AVS directory
        #[arg(
            long,
            value_name = "ADDR",
            env = "AVS_DIRECTORY_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        avs_directory: Option<Address>,
        /// The address of the rewards coordinator
        #[arg(
            long,
            value_name = "ADDR",
            env = "REWARDS_COORDINATOR_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        rewards_coordinator: Option<Address>,
        /// The address of the operator registry
        #[arg(
            long,
            value_name = "ADDR",
            env = "OPERATOR_REGISTRY_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        operator_registry: Option<Address>,
        /// The address of the network registry
        #[arg(
            long,
            value_name = "ADDR",
            env = "NETWORK_REGISTRY_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        network_registry: Option<Address>,
        /// The address of the base delegator
        #[arg(
            long,
            value_name = "ADDR",
            env = "BASE_DELEGATOR_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        base_delegator: Option<Address>,
        /// The address of the network opt-in service
        #[arg(
            long,
            value_name = "ADDR",
            env = "NETWORK_OPT_IN_SERVICE_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        network_opt_in_service: Option<Address>,
        /// The address of the vault opt-in service
        #[arg(
            long,
            value_name = "ADDR",
            env = "VAULT_OPT_IN_SERVICE_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        vault_opt_in_service: Option<Address>,
        /// The address of the slasher
        #[arg(
            long,
            value_name = "ADDR",
            env = "SLASHER_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        slasher: Option<Address>,
        /// The address of the veto slasher
        #[arg(
            long,
            value_name = "ADDR",
            env = "VETO_SLASHER_ADDRESS",
            required_if_eq("protocol", Protocol::Symbiotic.as_str())
        )]
        veto_slasher: Option<Address>,
    },
}

impl Default for GadgetCLICoreSettings {
    fn default() -> Self {
        Self::Run {
            target_addr: "127.0.0.1".parse().unwrap(),
            target_port: 8080,
            use_secure_url: false,
            test_mode: false,
            log_id: None,
            http_rpc_url: gadget_io::defaults::http_rpc_url(),
            ws_rpc_url: gadget_io::defaults::ws_rpc_url(),
            bootnodes: None,
            keystore_uri: String::new(),
            chain: SupportedChains::default(),
            verbose: 0,
            pretty: false,
            keystore_password: None,
            protocol: Protocol::default(),
            blueprint_id: Some(1),
            service_id: Some(1),
            skip_registration: false,
            registry_coordinator: None,
            operator_state_retriever: None,
            delegation_manager: None,
            service_manager: None,
            stake_registry: None,
            strategy_manager: None,
            avs_directory: None,
            rewards_coordinator: None,
            operator_registry: None,
            network_registry: None,
            base_delegator: None,
            network_opt_in_service: None,
            vault_opt_in_service: None,
            slasher: None,
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
        target_addr: IpAddr,
        target_port: u16,
        http_rpc_url: Url,
        ws_rpc_url: Url,
        use_secure_url: bool,
        skip_registration: bool,
        keystore_uri: String,
        keystore_password: Option<String>,
        chain: SupportedChains,
        protocol: Protocol,
        eigenlayer_contract_addresses: Option<EigenlayerContractAddresses>,
        symbiotic_contract_addresses: Option<SymbioticContractAddresses>,
        blueprint_id: Option<u64>,
        service_id: Option<u64>,
    ) -> Self {
        // Eigenlayer addresses
        let registry_coordinator =
            eigenlayer_contract_addresses.map(|a| a.registry_coordinator_address);
        let operator_state_retriever =
            eigenlayer_contract_addresses.map(|a| a.operator_state_retriever_address);
        let delegation_manager =
            eigenlayer_contract_addresses.map(|a| a.delegation_manager_address);
        let service_manager = eigenlayer_contract_addresses.map(|a| a.service_manager_address);
        let stake_registry = eigenlayer_contract_addresses.map(|a| a.stake_registry_address);
        let strategy_manager = eigenlayer_contract_addresses.map(|a| a.strategy_manager_address);
        let avs_directory = eigenlayer_contract_addresses.map(|a| a.avs_directory_address);
        let rewards_coordinator =
            eigenlayer_contract_addresses.map(|a| a.rewards_coordinator_address);

        // Symbiotic addresses
        let operator_registry = symbiotic_contract_addresses.map(|a| a.operator_registry_address);
        let network_registry = symbiotic_contract_addresses.map(|a| a.network_registry_address);
        let base_delegator = symbiotic_contract_addresses.map(|a| a.base_delegator_address);
        let network_opt_in_service =
            symbiotic_contract_addresses.map(|a| a.network_opt_in_service_address);
        let vault_opt_in_service =
            symbiotic_contract_addresses.map(|a| a.vault_opt_in_service_address);
        let slasher = symbiotic_contract_addresses.map(|a| a.slasher_address);
        let veto_slasher = symbiotic_contract_addresses.map(|a| a.veto_slasher_address);

        ContextConfig {
            gadget_core_settings: GadgetCLICoreSettings::Run {
                target_addr,
                target_port,
                use_secure_url,
                test_mode: false,
                log_id: None,
                http_rpc_url,
                bootnodes: None,
                keystore_uri,
                chain,
                verbose: 3,
                pretty: true,
                keystore_password,
                blueprint_id,
                service_id,
                skip_registration,
                protocol,
                registry_coordinator,
                operator_state_retriever,
                delegation_manager,
                ws_rpc_url,
                stake_registry,
                service_manager,
                strategy_manager,
                avs_directory,
                rewards_coordinator,
                operator_registry,
                network_registry,
                base_delegator,
                network_opt_in_service,
                vault_opt_in_service,
                slasher,
                veto_slasher,
            },
        }
    }

    /// Creates a new context config with the given parameters
    ///
    /// # Defaults
    /// - `target_addr`: The same host address as the given http_rpc_url, defaulting to 127.0.0.1 if an error occurs
    /// - `target_port`: The same port as the given http_rpc_url, defaulting to 0 if an error occurs
    /// - `use_secure_url`: false
    /// - `skip_registration`: false
    /// - `keystore_password`: None
    #[allow(clippy::too_many_arguments)]
    pub fn create_config_with_defaults(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        chain: SupportedChains,
        protocol: Protocol,
        eigenlayer_contract_addresses: Option<EigenlayerContractAddresses>,
        symbiotic_contract_addresses: Option<SymbioticContractAddresses>,
        blueprint_id: Option<u64>,
        service_id: Option<u64>,
    ) -> Self {
        let target_port = http_rpc_url.port().unwrap_or_default();
        let target_addr = http_rpc_url
            .host_str()
            .and_then(|host| IpAddr::from_str(host).ok())
            .unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

        ContextConfig::create_config(
            target_addr,
            target_port,
            http_rpc_url,
            ws_rpc_url,
            false,
            false,
            keystore_uri,
            None,
            chain,
            protocol,
            eigenlayer_contract_addresses,
            symbiotic_contract_addresses,
            blueprint_id,
            service_id,
        )
    }

    /// Creates a new context config with defaults for Eigenlayer
    pub fn create_eigenlayer_config(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        chain: SupportedChains,
        eigenlayer_contract_addresses: EigenlayerContractAddresses,
    ) -> Self {
        Self::create_config_with_defaults(
            http_rpc_url,
            ws_rpc_url,
            keystore_uri,
            chain,
            Protocol::Eigenlayer,
            Some(eigenlayer_contract_addresses),
            None,
            None,
            None,
        )
    }

    /// Creates a new context config with defaults for Symbiotic
    pub fn create_symbiotic_config(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        chain: SupportedChains,
        symbiotic_contract_addresses: SymbioticContractAddresses,
    ) -> Self {
        Self::create_config_with_defaults(
            http_rpc_url,
            ws_rpc_url,
            keystore_uri,
            chain,
            Protocol::Symbiotic,
            None,
            Some(symbiotic_contract_addresses),
            None,
            None,
        )
    }

    /// Creates a new context config with defaults for Tangle
    pub fn create_tangle_config(
        http_rpc_url: Url,
        ws_rpc_url: Url,
        keystore_uri: String,
        chain: SupportedChains,
    ) -> Self {
        Self::create_config_with_defaults(
            http_rpc_url,
            ws_rpc_url,
            keystore_uri,
            chain,
            Protocol::Tangle,
            None,
            None,
            None,
            None,
        )
    }
}

fn default_protocol() -> Protocol {
    Protocol::Tangle
}
