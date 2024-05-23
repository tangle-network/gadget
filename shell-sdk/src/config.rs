pub use gadget_io::KeystoreConfig;
use libp2p::Multiaddr;
use std::net::IpAddr;
use std::path::PathBuf;
use tangle_subxt::tangle_testnet_runtime::api::jobs::events::job_refunded::RoleType;

#[derive(Debug, Clone)]
pub struct ShellConfig<KBE: Clone> {
    pub base_path: PathBuf,
    pub keystore: KeystoreConfig,
    pub subxt: SubxtConfig,
    pub bind_ip: IpAddr,
    pub bind_port: u16,
    pub bootnodes: Vec<Multiaddr>,
    pub node_key: [u8; 32],
    pub role_types: Vec<RoleType>,
    pub n_protocols: usize,
    pub keystore_backend: KBE,
}

#[derive(Debug, Clone)]
pub struct SubxtConfig {
    /// The URL of the Tangle Node.
    pub endpoint: url::Url,
}
