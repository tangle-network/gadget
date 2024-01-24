use crate::protocols::keygen::MpEcdsaKeygenProtocol;
use crate::protocols::sign::MpEcdsaSigningProtocol;
use async_trait::async_trait;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient, PalletSubmitter};
use gadget_common::config::NetworkAndProtocolSetup;
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::network::Network;
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use pallet_jobs_rpc_runtime_api::JobsApi;
use protocol_macros::protocol;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::sync::Arc;

pub mod constants;
pub mod error;
pub mod protocols;
pub mod util;

#[protocol]
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

#[protocol]
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
    type Client = C;
    type Block = B;
    type Backend = BE;

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Block, Self::Backend, Self::Client>,
    ) -> Result<(Self::Network, Self::Protocol), gadget_common::Error> {
        let protocol = protocols::keygen::create_protocol(
            self.account_id,
            jobs_client,
            self.network.clone(),
            self.logger.clone(),
            self.keystore_backend.clone(),
        )
        .await;

        Ok((self.network.clone(), protocol))
    }

    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
        self.pallet_tx.clone()
    }

    fn logger(&self) -> DebugLogger {
        self.logger.clone()
    }

    fn client(&self) -> Self::Client {
        self.client.clone()
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
    type Client = C;
    type Block = B;
    type Backend = BE;

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Block, Self::Backend, Self::Client>,
    ) -> Result<(Self::Network, Self::Protocol), gadget_common::Error> {
        let protocol = protocols::sign::create_protocol(
            self.account_id,
            self.logger.clone(),
            jobs_client,
            self.network.clone(),
            self.keystore_backend.clone(),
        )
        .await;
        Ok((self.network.clone(), protocol))
    }

    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
        self.pallet_tx.clone()
    }

    fn logger(&self) -> DebugLogger {
        self.logger.clone()
    }

    fn client(&self) -> Self::Client {
        self.client.clone()
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run<B, BE, KBE, C, N, N2, Tx>(
    account_id: AccountId,
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
    let keygen_config = MpEcdsaKeygenProtocolConfig {
        account_id,
        network: network_keygen,
        keystore_backend: keystore.clone(),
        client: client_keygen,
        logger: logger.clone(),
        pallet_tx: pallet_tx.clone(),
        _pd: std::marker::PhantomData,
    };
    let keygen_future = keygen_config.execute();

    let sign_config = MpEcdsaSigningProtocolConfig {
        account_id,
        network: network_signing,
        keystore_backend: keystore,
        client: client_signing,
        logger,
        pallet_tx: pallet_tx.clone(),
        _pd: std::marker::PhantomData,
    };

    let sign_future = sign_config.execute();

    tokio::select! {
        res0 = keygen_future => res0,
        res1 = sign_future => res1,
    }
}
