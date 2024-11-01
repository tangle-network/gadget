use super::*;
use alloc::string::String;
use alloy_primitives::Address;
use core::fmt::Debug;
use core::net::IpAddr;
use gadget_io::SupportedChains;
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Default, Clone, clap::Parser, Serialize, Deserialize)]
#[command(name = "General CLI Context")]
#[cfg(feature = "std")]
pub struct ContextConfig {
    /// Pass through arguments to another command
    #[command(subcommand)]
    pub gadget_core_settings: GadgetCLICoreSettings,
}

#[derive(Debug, Clone, clap::Parser, Serialize, Deserialize)]
#[cfg(feature = "std")]
pub enum GadgetCLICoreSettings {
    #[command(name = "run")]
    Run {
        #[arg(long, short = 'b', env)]
        bind_addr: IpAddr,
        #[arg(long, short = 'p', env)]
        bind_port: u16,
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
        /// The address of the AVS directory
        #[arg(
            long,
            value_name = "ADDR",
            env = "AVS_DIRECTORY_ADDRESS",
            required_if_eq("protocol", Protocol::Eigenlayer.as_str())
        )]
        avs_directory: Option<Address>,
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
            bind_addr: "127.0.0.1".parse().unwrap(),
            bind_port: 8080,
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
            strategy_manager: None,
            avs_directory: None,
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

fn default_protocol() -> Protocol {
    Protocol::Tangle
}
