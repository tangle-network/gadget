use crate::util::DebugLogger;
use std::error::Error;
use std::net::SocketAddr;

pub mod application;
pub mod client;
pub mod constants;
pub mod error;
pub mod keystore;
pub mod network;
pub mod protocols;
pub mod util;

pub struct MpEcdsaProtocolConfig {
    // Set to some if a peer connection to the target bootnode
    pub gossip_bootnode: Option<SocketAddr>,
    // Set to some if the bootnode
    pub gossip_bind_addr: Option<SocketAddr>,
}

pub async fn run_keygen(config: &MpEcdsaProtocolConfig) -> Result<(), Box<dyn Error>> {
    let debug_logger = DebugLogger;
    let client = client::create_client(config).await?;
    let network = network::create_network(debug_logger.clone(), config).await?;
    let protocol = protocols::keygen::create_protocol(config).await?;

    webb_gadget::run_protocol(network, protocol, client).await?;
    Ok(())
}

pub async fn run_sign(config: &MpEcdsaProtocolConfig) -> Result<(), Box<dyn Error>> {
    let client = client::create_client(config).await?;
    let network = network::create_network(config).await?;
    let protocol = protocols::sign::create_protocol(config).await?;

    webb_gadget::run_protocol(network, protocol, client).await?;
    Ok(())
}

pub async fn run(config: MpEcdsaProtocolConfig) -> Result<(), Box<dyn Error>> {
    let keygen_future = run_keygen(&config);
    let sign_future = run_sign(&config);

    tokio::select! {
        res0 = keygen_future => res0,
        res1 = sign_future => res1,
    }
}
