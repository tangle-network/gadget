use crate::protocols::keygen::MpEcdsaKeygenProtocol;
use crate::protocols::sign::MpEcdsaSigningProtocol;
use async_trait::async_trait;
use gadget_common::client::{AccountId, ClientWithApi, PalletSubmitter};
use gadget_common::config::NetworkAndProtocolSetup;
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::network::Network;
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use pallet_jobs_rpc_runtime_api::JobsApi;
use protocol_macros::define_protocol;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::sync::Arc;

pub mod constants;
pub mod error;
pub mod protocols;
pub mod util;

#[define_protocol(KeygenProtocol)]
pub struct MpEcdsaKeygenProtocolConfig<
    N: Network,
    B: Block,
    BE: Backend<B>,
    KBE: KeystoreBackend,
    C: ClientWithApi<B, BE>,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pub account_id: AccountId,
    pub network: N,
    pub keystore_backend: ECDSAKeyStore<KBE>,
    pub client: C,
    pub logger: DebugLogger,
    pub pallet_tx: Arc<dyn PalletSubmitter>,
    pub _pd: std::marker::PhantomData<(B, BE)>,
}

#[define_protocol(SigningProtocol)]
pub struct MpEcdsaSigningProtocolConfig<
    N: Network,
    B: Block,
    BE: Backend<B>,
    KBE: KeystoreBackend,
    C: ClientWithApi<B, BE>,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pub account_id: AccountId,
    pub network: N,
    pub keystore_backend: ECDSAKeyStore<KBE>,
    pub client: C,
    pub logger: DebugLogger,
    pub pallet_tx: Arc<dyn PalletSubmitter>,
    pub _pd: std::marker::PhantomData<(B, BE)>,
}

#[async_trait]
impl<N: Network, B: Block, BE: Backend<B>, KBE: KeystoreBackend, C: ClientWithApi<B, BE>>
    NetworkAndProtocolSetup for MpEcdsaKeygenProtocolConfig<N, B, BE, KBE, C>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type Network = N;
    type Protocol = MpEcdsaKeygenProtocol<B, BE, KBE, C, N>;
    async fn build_network(&self) -> Result<Self::Network, gadget_common::Error> {
        Ok(self.network.clone())
    }
    async fn build_protocol(&self) -> Result<Self::Protocol, gadget_common::Error> {
        let client = gadget_common::client::create_client(
            self.client.clone(),
            self.logger.clone(),
            self.pallet_tx.clone(),
        )
        .await?;
        let protocol = protocols::keygen::create_protocol(
            self.account_id,
            client.clone(),
            self.network.clone(),
            self.logger.clone(),
            self.keystore_backend.clone(),
        )
        .await;
        Ok(protocol)
    }
}

#[async_trait]
impl<N: Network, B: Block, BE: Backend<B>, KBE: KeystoreBackend, C: ClientWithApi<B, BE>>
    NetworkAndProtocolSetup for MpEcdsaSigningProtocolConfig<N, B, BE, KBE, C>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type Network = N;
    type Protocol = MpEcdsaSigningProtocol<B, BE, KBE, C, N>;
    async fn build_network(&self) -> Result<Self::Network, gadget_common::Error> {
        Ok(self.network.clone())
    }
    async fn build_protocol(&self) -> Result<Self::Protocol, gadget_common::Error> {
        let client = gadget_common::client::create_client(
            self.client.clone(),
            self.logger.clone(),
            self.pallet_tx.clone(),
        )
        .await?;
        let protocol = protocols::sign::create_protocol(
            self.account_id,
            self.logger.clone(),
            client.clone(),
            self.network.clone(),
            self.keystore_backend.clone(),
        )
        .await;
        Ok(protocol)
    }
}
/*
pub async fn run_keygen<B, BE, KBE, C, N>(
    config: &MpEcdsaProtocolConfig,
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
    /*
    let client =
        gadget_common::client::create_client(client_inner.clone(), logger.clone(), pallet_tx)
            .await?;
    let protocol = protocols::keygen::create_protocol(
        config,
        client.clone(),
        network.clone(),
        logger.clone(),
        keystore,
    )
    .await;

    logger.info("Done creating keygen protocol");

    let protocol = EcdsaProtocolConfig::new(network, client_inner, protocol);

    gadget_common::run_protocol(protocol).await?;*/
    Ok(())
}

pub async fn run_sign<B, BE, KBE, C, N>(
    config: &MpEcdsaProtocolConfig,
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
/*
    let client =
        gadget_common::client::create_client(client_inner.clone(), logger.clone(), pallet_tx)
            .await?;
    let protocol = protocols::sign::create_protocol(
        config,
        logger.clone(),
        client.clone(),
        network.clone(),
        keystore,
    )
    .await;

    logger.info("Done creating sign protocol");

    let protocol = EcdsaProtocolConfig::from(config.clone()).build().await?;

    gadget_common::run_protocol(protocol).await?;*/
    Ok(())
}
/*
#[allow(clippy::too_many_arguments)]
pub async fn run<B, BE, KBE, C, N, N2, Tx>(
    config: MpEcdsaProtocolConfig,
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

    let sign_future = run_sign(
        &config,
        client_signing,
        logger,
        keystore,
        network_signing,
        pallet_tx,
    );

    tokio::select! {
        res0 = keygen_future => res0,
        res1 = sign_future => res1,
    }
}
*/*/
