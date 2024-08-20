use crate::shared::shell::SupportedChains;
use core::net::IpAddr;
use multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::path::PathBuf;
    } else {
        use alloc::string::String;
        use alloc::vec::Vec;
        #[cfg(feature = "typescript")]
        use alloc::string::ToString;
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "typescript", derive(tsify::Tsify))]
#[cfg_attr(feature = "typescript", tsify(from_wasm_abi))]
pub struct Opt {
    /// The verbosity level, can be used multiple times
    pub verbose: i32,
    /// Whether to use pretty logging
    pub pretty: bool,
    /// The options for the gadget
    pub options: GadgetTomlConfig,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(tsify::Tsify))]
#[cfg_attr(feature = "typescript", tsify(from_wasm_abi))]
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
    /// Keystore Password, if not provided, the password will be read from the environment variable.
    pub keystore_password: Option<String>,
    /// The chain to connect to, must be one of the supported chains.
    pub chain: SupportedChains,
    /// The verbosity level, can be used multiple times
    pub verbose: i32,
    /// Whether to use pretty logging
    pub pretty: bool,
}
