use libp2p::Multiaddr;
use sp_core::crypto::SecretString;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ShellConfig {
    pub keystore: KeystoreConfig,
    pub subxt: SubxtConfig,
    pub bind_ip: String,
    pub bind_port: u16,
    pub bootnodes: Vec<Multiaddr>,
    pub genesis_hash: [u8; 32],
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
