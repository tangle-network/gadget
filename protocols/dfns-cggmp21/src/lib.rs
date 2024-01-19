use gadget_common::client::{AccountId, ClientWithApi, PalletSubmitter};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::network::Network;
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use pallet_jobs_rpc_runtime_api::JobsApi;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::sync::Arc;

pub mod constants;
pub mod error;
pub mod protocols;
pub mod util;

#[derive(Clone)]
pub struct DfnsCGGMP21ProtocolConfig {
    pub account_id: AccountId,
}

pub async fn run_keygen<B, BE, KBE, C, N>(
    config: &DfnsCGGMP21ProtocolConfig,
    client_inner: C,
    logger: DebugLogger,
    keystore: ECDSAKeyStore<KBE>,
    network: N,
    pallet_tx: Arc<dyn PalletSubmitter>,
) -> Result<(), gadget_common::Error>
where
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    logger.info("Starting keygen protocol");

    let client_inner = Arc::new(client_inner);
    let client =
        gadget_common::client::create_client(client_inner.clone(), logger.clone(), pallet_tx)
            .await?;
    let protocol = protocols::keygen::create_protocol(
        config,
        client,
        network.clone(),
        logger.clone(),
        keystore,
    )
    .await;

    logger.info("Done creating keygen protocol");

    gadget_common::run_protocol(network, protocol, client_inner).await?;
    Ok(())
}

pub async fn run_sign<B, BE, KBE, C, N>(
    config: &DfnsCGGMP21ProtocolConfig,
    client_inner: C,
    logger: DebugLogger,
    keystore: ECDSAKeyStore<KBE>,
    network: N,
    pallet_tx: Arc<dyn PalletSubmitter>,
) -> Result<(), gadget_common::Error>
where
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    logger.info("Starting sign protocol");

    let client_inner = Arc::new(client_inner);
    let client =
        gadget_common::client::create_client(client_inner.clone(), logger.clone(), pallet_tx)
            .await?;
    // let protocol =
    //     protocols::sign::create_protocol(config, logger.clone(), client, network.clone(), keystore)
    //         .await;
    let protocol = ();

    logger.info("Done creating sign protocol");

    gadget_common::run_protocol(network, protocol, client_inner).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn run<B, BE, KBE, C, N, N2, Tx>(
    config: DfnsCGGMP21ProtocolConfig,
    client_keygen: C,
    client_signing: C,
    logger: DebugLogger,
    keystore: ECDSAKeyStore<KBE>,
    network_keygen: N,
    network_signing: N2,
    pallet_tx: Tx,
) -> Result<(), gadget_common::Error>
where
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    N2: Network,
    Tx: PalletSubmitter,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    let pallet_tx = Arc::new(pallet_tx) as Arc<dyn PalletSubmitter>;
    let keygen_future = run_keygen(
        &config,
        client_keygen,
        logger.clone(),
        keystore.clone(),
        network_keygen,
        pallet_tx.clone(),
    );

    // let sign_future = run_sign(
    //     &config,
    //     client_signing,
    //     logger,
    //     keystore,
    //     network_signing,
    //     pallet_tx,
    // );
    //
    let sign_future = futures::future::pending::<_>();

    tokio::select! {
        res0 = keygen_future => res0,
        res1 = sign_future => res1,
    }
}
