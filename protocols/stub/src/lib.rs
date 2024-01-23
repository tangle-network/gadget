use crate::protocol::StubProtocol;
use async_trait::async_trait;
use gadget_common::client::*;
use gadget_common::config::*;
use gadget_common::Error;
use network::StubNetworkService;
use protocol_macros::protocol;
use std::sync::Arc;

pub mod network;
pub mod protocol;

#[protocol]
pub struct StubConfig<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
    client: C,
    _pd: std::marker::PhantomData<(B, BE)>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>> NetworkAndProtocolSetup
    for StubConfig<B, BE, C>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type Network = StubNetworkService;
    type Protocol = StubProtocol<B, BE, C>;
    type Client = C;
    type Block = B;
    type Backend = BE;

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Block, Self::Backend, Self::Client>,
    ) -> Result<(Self::Network, Self::Protocol), Error> {
        let stub_protocol = StubProtocol {
            jobs_client,
            account_id: AccountId::from_raw([0u8; 33]),
            logger: self.logger.clone(),
        };

        Ok((StubNetworkService, stub_protocol))
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

pub async fn run<B: Block, BE: Backend<B> + 'static, C: ClientWithApi<B, BE>>(
    client: C,
    pallet_tx: Arc<dyn PalletSubmitter>,
    logger: DebugLogger,
) -> Result<(), Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    let config = StubConfig {
        pallet_tx,
        logger,
        client,
        _pd: std::marker::PhantomData,
    };

    config.execute().await
}
