use gadget_common::client::{AccountId, ClientWithApi};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::network::Network;
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use pallet_jobs_rpc_runtime_api::JobsApi;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::error::Error;
use std::sync::Arc;

pub mod constants;
pub mod error;
pub mod mock;
pub mod network;
pub mod protocols;
pub mod util;

pub struct MpEcdsaProtocolConfig {
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
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, gadget_common::client::AccountId>,
{
    let client =
        gadget_common::client::create_client(client_inner.clone(), logger.clone(), keystore)
            .await?;
    let protocol =
        protocols::keygen::create_protocol(config, client, network.clone(), logger).await?;

    gadget_common::run_protocol(network, protocol, client_inner).await?;
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
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, gadget_common::client::AccountId>,
{
    let client =
        gadget_common::client::create_client(client_inner.clone(), logger.clone(), keystore)
            .await?;
    let protocol =
        protocols::sign::create_protocol(config, logger, client, network.clone()).await?;

    gadget_common::run_protocol(network, protocol, client_inner).await?;
    Ok(())
}

pub async fn run<B, BE, KBE, C, N, N2>(
    config: MpEcdsaProtocolConfig,
    client: C,
    logger: DebugLogger,
    keystore: ECDSAKeyStore<KBE>,
    network_keygen: N,
    network_signing: N2,
) -> Result<(), Box<dyn Error>>
where
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    N2: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, gadget_common::client::AccountId>,
{
    let client = Arc::new(client);
    let keygen_future = run_keygen(
        &config,
        client.clone(),
        logger.clone(),
        keystore.clone(),
        network_keygen,
    );
    let sign_future = run_sign(&config, client, logger, keystore, network_signing);

    tokio::select! {
        res0 = keygen_future => res0,
        res1 = sign_future => res1,
    }
}
