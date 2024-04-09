use libp2p::Multiaddr;
use sp_core::crypto::SecretString;
use std::path::{Path, PathBuf};
use gadget_io::keystore::KeystoreConfig;

#[derive(Debug, Clone)]
pub struct ShellConfig {
    pub base_path: PathBuf,
    pub keystore: KeystoreConfig,
    // #[cfg(not(family = "wasm"))]
    // pub keystore: KeystoreConfig,
    // #[cfg(family = "wasm")]
    // pub keystore: WasmConfig,
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