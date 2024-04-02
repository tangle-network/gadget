use libp2p::Multiaddr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ShellConfig {
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

/// Configuration of the client keystore.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum KeystoreConfig {
    // Keystore at a path on-disk. Recommended for native gadgets.
    // Path {
    //     /// The path of the keystore.
    //     path: PathBuf,
    //     /// keystore's password.
    //     password: Option<SecretString>,
    // },
    /// In-memory keystore.
    InMemory,
}

impl KeystoreConfig {
    /// Returns None. Keystore other than InMemory not yet implemented for Web
    #[allow(dead_code)]
    pub fn path(&self) -> Option<&Path> {
        None
    }
}