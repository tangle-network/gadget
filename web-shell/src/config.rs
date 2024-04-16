use libp2p::Multiaddr;
use std::path::{Path, PathBuf};
pub use gadget_io::{KeystoreConfig, SubstrateKeystore};

#[derive(Debug, Clone)]
pub struct ShellConfig{
    pub base_path: PathBuf,
    pub keystore: KeystoreConfig,
    pub subxt: SubxtConfig,
    pub bind_ip: std::net::IpAddr,
    pub bind_port: u16,
    pub bootnodes: Vec<Multiaddr>,
    pub node_key: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct SubxtConfig {
    /// The URL of the Tangle Node.
    pub endpoint: url::Url,
}
