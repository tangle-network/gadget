use crate::client::{AccountId, ClientWithApi};
use crate::keystore::{ECDSAKeyStore, KeystoreBackend};
use crate::util::DebugLogger;
use pallet_jobs_rpc_runtime_api::JobsApi;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use webb_gadget::gadget::network::Network;

#[cfg(feature = "application")]
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
    pub account_id: AccountId,
}

pub async fn run_keygen<B, BE, KBE, C, N>(
    config: &MpEcdsaProtocolConfig,
    client_inner: Arc<C>,
    logger: DebugLogger,
    keystore: ECDSAKeyStore<KBE>,
    network: N,
) -> Result<(), Box<dyn Error>>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    let client =
        client::create_client(config, client_inner.clone(), logger.clone(), keystore).await?;
    let protocol =
        protocols::keygen::create_protocol(config, client, network.clone(), logger).await?;

    webb_gadget::run_protocol(network, protocol, client_inner).await?;
    Ok(())
}

pub async fn run_sign<B, BE, KBE, C, N>(
    config: &MpEcdsaProtocolConfig,
    client_inner: Arc<C>,
    logger: DebugLogger,
    keystore: ECDSAKeyStore<KBE>,
    network: N,
) -> Result<(), Box<dyn Error>>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    let client =
        client::create_client(config, client_inner.clone(), logger.clone(), keystore).await?;
    let protocol =
        protocols::sign::create_protocol(config, logger, client, network.clone()).await?;

    webb_gadget::run_protocol(network, protocol, client_inner).await?;
    Ok(())
}

pub async fn run<B, BE, KBE, C, N>(
    config: MpEcdsaProtocolConfig,
    client: C,
    logger: DebugLogger,
    keystore: ECDSAKeyStore<KBE>,
    network: N,
) -> Result<(), Box<dyn Error>>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    let client = Arc::new(client);
    let keygen_future = run_keygen(
        &config,
        client.clone(),
        logger.clone(),
        keystore.clone(),
        network.clone(),
    );
    let sign_future = run_sign(&config, client, logger, keystore, network);

    tokio::select! {
        res0 = keygen_future => res0,
        res1 = sign_future => res1,
    }
}
