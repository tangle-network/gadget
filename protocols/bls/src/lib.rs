/*use crate::protocol::keygen::BlsKeygenProtocol;
use crate::protocol::signing::BlsSigningProtocol;
use async_trait::async_trait;
use gadget_common::client::*;
use gadget_common::config::*;
use gadget_common::keystore::{GenericKeyStore, KeystoreBackend};
use gadget_common::Error;
use protocol_macros::protocol;
use std::sync::Arc;

pub mod keygen;
pub mod signing;
pub mod protocol;

#[protocol]
pub struct BlsKeygenConfig<
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
    client: C,
    network: N,
    key_store: GenericKeyStore<KBE, gadget_common::sp_core::ecdsa::Pair>,
    _pd: std::marker::PhantomData<(B, BE)>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, N: Network, KBE: KeystoreBackend>
    NetworkAndProtocolSetup for BlsKeygenConfig<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type Network = N;
    type Protocol = BlsKeygenProtocol<B, BE, C, N, KBE>;
    type Client = C;
    type Block = B;
    type Backend = BE;

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Block, Self::Backend, Self::Client>,
    ) -> Result<(Self::Network, Self::Protocol), Error> {
        let protocol = BlsKeygenProtocol {
            jobs_client,
            account_id: AccountId::from_raw([0u8; 33]),
            logger: self.logger.clone(),
            network: self.network.clone(),
            pallet_tx: self.pallet_tx.clone(),
            keystore: self.key_store.clone(),
        };

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

#[protocol]
pub struct BlsSigningConfig<
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
    client: C,
    network: N,
    key_store: GenericKeyStore<KBE, gadget_common::sp_core::ecdsa::Pair>,
    _pd: std::marker::PhantomData<(B, BE)>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, N: Network, KBE: KeystoreBackend>
    NetworkAndProtocolSetup for BlsSigningConfig<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type Network = N;
    type Protocol = BlsSigningProtocol<B, BE, C, N, KBE>;
    type Client = C;
    type Block = B;
    type Backend = BE;

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Block, Self::Backend, Self::Client>,
    ) -> Result<(Self::Network, Self::Protocol), Error> {
        let protocol = BlsSigningProtocol {
            jobs_client,
            account_id: AccountId::from_raw([0u8; 33]),
            logger: self.logger.clone(),
            network: self.network.clone(),
            keystore: self.key_store.clone(),
        };

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

pub async fn run<
    B: Block,
    BE: Backend<B> + 'static,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
>(
    client_keygen: C,
    client_signing: C,
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
    network_keygen: N,
    network_signing: N,
    keystore: GenericKeyStore<KBE, gadget_common::sp_core::ecdsa::Pair>,
) -> Result<(), Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    let config_keygen = BlsKeygenConfig {
        pallet_tx: pallet_tx.clone(),
        logger: logger.clone(),
        client: client_keygen,
        network: network_keygen,
        _pd: std::marker::PhantomData,
        key_store: keystore.clone(),
    };

    let config_signing = BlsSigningConfig {
        pallet_tx,
        logger,
        client: client_signing,
        network: network_signing,
        _pd: std::marker::PhantomData,
        key_store: keystore.clone(),
    };

    let keygen_future = config_keygen.execute();
    let signing_future = config_signing.execute();

    tokio::select! {
        res0 = keygen_future => res0,
        res1 = signing_future => res1,
    }
}
*/
pub mod protocol;
