use std::error::Error;

pub mod client;
pub mod network;
pub mod protocols;
pub mod util;
pub mod constants;
pub mod keystore;
pub mod keyring;
pub mod indexing;
pub mod error;

pub struct MpEcdsaProtocolConfig {
    pub gossip_bootnode: String,
}

pub async fn run_keygen(config: &MpEcdsaProtocolConfig) -> Result<(), Box<dyn Error>> {
    let client = client::create_client(config).await?;
    let network = network::create_network(config).await?;
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
