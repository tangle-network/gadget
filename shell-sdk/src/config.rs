use gadget_common::environments::GadgetEnvironment;
pub use gadget_io::KeystoreConfig;
use libp2p::Multiaddr;
use std::net::IpAddr;
use std::path::PathBuf;
use tangle_primitives::roles::RoleType;

#[derive(Debug)]
pub struct ShellConfig<KBE: Clone, Env: GadgetEnvironment> {
    pub base_path: PathBuf,
    pub keystore: KeystoreConfig,
    pub environment: Env,
    pub bind_ip: IpAddr,
    pub bind_port: u16,
    pub bootnodes: Vec<Multiaddr>,
    pub node_key: [u8; 32],
    pub role_types: Vec<RoleType>,
    pub n_protocols: usize,
    pub keystore_backend: KBE,
}
