use crate::shared::shell::SupportedChains;
use clap::Parser;
use multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, path::PathBuf};

#[derive(Debug, Parser)]
#[command(
    name = "Gadget",
    about = "An MPC executor that connects to the Tangle network to perform work"
)]
pub struct Opt {
    /// The path to the configuration file. If not provided, the default configuration will be used.
    /// Note that if the configuration file is provided, the command line arguments will be ignored.
    #[arg(global = true, short = 'c', long = "config")]
    pub config: Option<PathBuf>,
    /// The options for the shell
    #[command(flatten)]
    pub options: GadgetConfig,
}

#[derive(Debug, Parser, Serialize, Deserialize)]
/// All shells should expect this as CLI input. The Blueprint Manager will be responsible for passing these values to this gadget binary
pub struct GadgetConfig {
    /// The IP address to bind to for the libp2p node.
    #[arg(long = "bind-ip", short = 'i', default_value = defaults::BIND_IP)]
    #[serde(default = "defaults::bind_ip")]
    pub bind_addr: IpAddr,
    /// The port to bind to for the libp2p node.
    #[arg(long = "port", short = 'p', default_value = defaults::BIND_PORT)]
    #[serde(default = "defaults::bind_port")]
    pub bind_port: u16,
    /// The HTTP RPC URL of the Tangle Node.
    #[arg(long = "url", value_parser = url::Url::parse, default_value = defaults::HTTP_RPC_URL)]
    #[serde(default = "defaults::http_rpc_url")]
    pub http_rpc_url: url::Url,
    /// The WS RPC URL of the Tangle Node.
    #[arg(long = "url", value_parser = url::Url::parse, default_value = defaults::WS_RPC_URL)]
    #[serde(default = "defaults::ws_rpc_url")]
    pub ws_rpc_url: url::Url,
    /// The List of bootnodes to connect to
    #[arg(long = "bootnodes", value_parser  = <Multiaddr as std::str::FromStr>::from_str, action = clap::ArgAction::Append)]
    #[serde(default)]
    pub bootnodes: Vec<Multiaddr>,
    /// The base path to store the blueprint manager data, and read data from the keystore.
    #[arg(long, short = 'd')]
    pub keystore_uri: String,
    /// Keystore Password, if not provided, the password will be read from the environment variable.
    #[arg(long = "keystore-password", env)]
    pub keystore_password: Option<String>,
    /// The chain to connect to, must be one of the supported chains.
    #[arg(long, default_value_t = SupportedChains::LocalTestnet)]
    #[serde(default)]
    pub chain: SupportedChains,
    /// The verbosity level, can be used multiple times
    #[arg(long, short = 'v', global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Whether to use pretty logging
    #[arg(global = true, long)]
    pub pretty: bool,
}

pub mod defaults {
    pub const BIND_PORT: &str = "30555";
    pub const BIND_IP: &str = "0.0.0.0";
    pub const HTTP_RPC_URL: &str = "http://127.0.0.1:9944";
    pub const WS_RPC_URL: &str = "ws://127.0.0.1:9944";

    pub fn http_rpc_url() -> url::Url {
        url::Url::parse(HTTP_RPC_URL).expect("Default RPC URL is valid")
    }

    pub fn ws_rpc_url() -> url::Url {
        url::Url::parse(WS_RPC_URL).expect("Default RPC URL is valid")
    }

    pub fn bind_ip() -> std::net::IpAddr {
        BIND_IP.parse().expect("Default bind IP is valid")
    }

    pub fn bind_port() -> u16 {
        BIND_PORT.parse().expect("Default bind port is valid")
    }
}
