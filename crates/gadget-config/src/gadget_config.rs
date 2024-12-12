use super::*;
use core::fmt::Debug;
use std::path::PathBuf;

#[cfg(feature = "networking")]
use libp2p::Multiaddr;

pub type StdGadgetConfiguration = GadgetConfiguration;

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
    /// The list of bootnodes to connect to
    #[cfg(feature = "networking")]
    pub bootnodes: Vec<Multiaddr>,
    /// The type of protocol the gadget is executing on.
    pub protocol: Protocol,
    /// Protocol-specific settings
    pub protocol_settings: ProtocolSettings,
    /// Whether the gadget is in test mode
    pub test_mode: bool,
}
