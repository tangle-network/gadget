use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::PathBuf, str::FromStr};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize, Debug, Tsify)]
#[tsify(from_wasm_abi)]
pub struct Opt {
    /// The path to the configuration file. If not provided, the default configuration will be used.
    /// Note that if the configuration file is provided, the command line arguments will be ignored.
    pub config: Option<PathBuf>,
    /// The verbosity level, can be used multiple times
    pub verbose: i32,
    /// Whether to use pretty logging
    pub pretty: bool,
    /// The options for the gadget
    pub options: GadgetTomlConfig,
}

#[derive(Default, Debug, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "snake_case")]
#[tsify(from_wasm_abi)]
pub enum SupportedChains {
    #[default]
    LocalTestnet,
    LocalMainnet,
    Testnet,
    Mainnet,
}

#[derive(Debug, Serialize, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
/// All gadgets should expect this as CLI input. The Blueprint Manager will be responsible for passing these values to the gadget binary.
pub struct GadgetTomlConfig {
    /// The IP address to bind to for the libp2p node.
    pub bind_ip: IpAddr,
    /// The port to bind to for the libp2p node.
    pub bind_port: u16,
    /// The RPC URL of the Tangle Node.
    pub url: url::Url,
    /// The List of bootnodes to connect to
    pub bootnodes: Vec<Multiaddr>,
    /// The base path to store the blueprint manager data, and read data from the keystore.
    pub base_path: PathBuf,
    /// Keystore Password, if not provided, the password will be read from the environment variable.
    pub keystore_password: Option<String>,
    /// The chain to connect to, must be one of the supported chains.
    pub chain: SupportedChains,
    /// The verbosity level, can be used multiple times
    pub verbose: i32,
    /// Whether to use pretty logging
    pub pretty: bool,
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
