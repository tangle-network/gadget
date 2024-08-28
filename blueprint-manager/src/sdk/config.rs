use gadget_common::environments::GadgetEnvironment;
use gadget_sdk::io::KeystoreConfig;
use libp2p::Multiaddr;
use std::net::IpAddr;
use std::path::PathBuf;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::ServiceBlueprint;

#[derive(Debug)]
pub struct SingleGadgetConfig<KBE: Clone, Env: GadgetEnvironment> {
    pub base_path: PathBuf,
    pub keystore: KeystoreConfig,
    pub environment: Env,
    pub bind_ip: IpAddr,
    pub bind_port: u16,
    pub bootnodes: Vec<Multiaddr>,
    pub services: Vec<ServiceBlueprint>,
    pub n_protocols: usize,
    pub keystore_backend: KBE,
}
