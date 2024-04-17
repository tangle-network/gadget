use std::net::IpAddr;
use libp2p::Multiaddr;
use sp_core::crypto::SecretString;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(Debug, Clone)]
pub struct ShellConfig {
    pub base_path: PathBuf,
    pub keystore: KeystoreConfig,
    pub subxt: SubxtConfig,
    pub bind_ip: IpAddr,
    pub bind_port: u16,
    pub bootnodes: Vec<Multiaddr>,
    pub node_key: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct SubxtConfig {
    /// The URL of the Tangle Node.
    pub endpoint: url::Url,
}

/// Configuration of the client keystore.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum KeystoreConfig {
    /// Keystore at a path on-disk. Recommended for native gadgets.
    Path {
        /// The path of the keystore.
        path: PathBuf,
        /// keystore's password.
        password: Option<SecretString>,
    },
    /// In-memory keystore.
    InMemory,
}

impl KeystoreConfig {
    /// Returns the path for the keystore.
    #[allow(dead_code)]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Path { path, .. } => Some(path),
            Self::InMemory => None,
        }
    }
}

#[derive(Debug, StructOpt, Serialize, Deserialize)]
struct TomlConfig {
    /// The IP address to bind to for the libp2p node.
    #[structopt(long = "bind-ip", short = "i", default_value = defaults::BIND_IP)]
    #[serde(default = "defaults::bind_ip")]
    bind_ip: IpAddr,
    /// The port to bind to for the libp2p node.
    #[structopt(long = "port", short = "p", default_value = defaults::BIND_PORT)]
    #[serde(default = "defaults::bind_port")]
    bind_port: u16,
    /// The RPC URL of the Tangle Node.
    #[structopt(long = "url", parse(try_from_str = url::Url::parse), default_value = defaults::RPC_URL)]
    #[serde(default = "defaults::rpc_url")]
    url: url::Url,
    /// The List of bootnodes to connect to
    #[structopt(long = "bootnodes", parse(try_from_str = <Multiaddr as std::str::FromStr>::from_str))]
    #[serde(default)]
    bootnodes: Vec<Multiaddr>,
    /// The node key in hex format. If not provided, a random node key will be generated.
    #[structopt(long = "node-key", env, parse(try_from_str = parse_node_key))]
    #[serde(skip_serializing)]
    node_key: Option<String>,
    /// The base path to store the shell-sdk data, and read data from the keystore.
    #[structopt(
    parse(from_os_str),
    long,
    short = "d",
    required_unless = "config",
    default_value_if("config", None, ".")
    )]
    base_path: PathBuf,
    /// Keystore Password, if not provided, the password will be read from the environment variable.
    #[structopt(long = "keystore-password", env)]
    keystore_password: Option<String>,
    /// The chain to connect to, must be one of the supported chains.
    #[structopt(
    long,
    default_value,
    possible_values = &[
    "local_testnet",
    "local_mainnet",
    "testnet",
    "mainnet"
    ]
    )]
    #[serde(default)]
    chain: SupportedChains,
}

#[derive(Default, Debug, StructOpt, Serialize, Deserialize)]
#[structopt(rename_all = "snake_case")]
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

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupportedChains::LocalTestnet => write!(f, "local_testnet"),
            SupportedChains::LocalMainnet => write!(f, "local_mainnet"),
            SupportedChains::Testnet => write!(f, "testnet"),
            SupportedChains::Mainnet => write!(f, "mainnet"),
        }
    }
}

pub mod defaults {
    pub const BIND_PORT: &str = "30555";
    pub const BIND_IP: &str = "0.0.0.0";
    pub const RPC_URL: &str = "ws://127.0.0.1:9944";

    pub fn rpc_url() -> url::Url {
        url::Url::parse(RPC_URL).expect("Default RPC URL is valid")
    }

    pub fn bind_ip() -> std::net::IpAddr {
        BIND_IP.parse().expect("Default bind IP is valid")
    }

    pub fn bind_port() -> u16 {
        BIND_PORT.parse().expect("Default bind port is valid")
    }

    /// Generates a random node key
    pub fn generate_node_key() -> [u8; 32] {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let mut array = [0u8; 32];
        rng.fill(&mut array);
        array
    }
}

fn parse_node_key(s: &str) -> color_eyre::Result<String> {
    let result: [u8; 32] = hex::decode(s.replace("0x", ""))?.try_into().map_err(|_| {
        color_eyre::eyre::eyre!("Invalid node key length, expect 32 bytes hex string")
    })?;
    Ok(hex::encode(result))
}
