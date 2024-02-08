use crate::client::{create_client, JobsApiForGadget, JobsClient, PalletSubmitter};
pub use crate::client::{AccountId, ClientWithApi};
pub use crate::debug_logger::DebugLogger;
pub use crate::gadget::network::Network;
pub use crate::gadget::GadgetProtocol;
use async_trait::async_trait;
pub use pallet_jobs_rpc_runtime_api::JobsApi;
pub use sc_client_api::Backend;
pub use sp_api::ProvideRuntimeApi;
pub use sp_runtime::traits::Block;
use std::sync::Arc;

#[async_trait]
pub trait ProtocolConfig
where
    <<Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Client as ProvideRuntimeApi<<Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Block>>::Api: JobsApiForGadget<<Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Block>,
    Self: Sized,
{
    type Network: Network;
    type Protocol: GadgetProtocol<
        <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Block,
        <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Backend,
        <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Client
    >;
    type ProtocolSpecificConfiguration: Clone
        + NetworkAndProtocolSetup<Network = Self::Network, Protocol = Self::Protocol> + Sync;
    fn params(&self) -> &Self::ProtocolSpecificConfiguration;

    fn take_network(&mut self) -> Self::Network;
    fn take_protocol(&mut self) -> Self::Protocol;
    fn take_client(&mut self) -> <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Client;

    async fn build(&self) -> Result<Self, crate::Error> {
        let jobs_client = self.params().build_jobs_client().await?;
        let (network, protocol) = self.params().build_network_and_protocol(jobs_client).await?;
        let client = self.params().client();
        let params = self.params().clone();
        let pallet_tx = self.pallet_tx();
        let logger = self.logger();
        Ok(Self::new(
            network, client, protocol, params, pallet_tx, logger,
        ))
    }

    fn new(
        network: Self::Network,
        client: <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Client,
        protocol: Self::Protocol,
        params: Self::ProtocolSpecificConfiguration,
        pallet_tx: Arc<dyn PalletSubmitter>,
        logger: DebugLogger,
    ) -> Self;

    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
        self.params().pallet_tx()
    }
    fn logger(&self) -> DebugLogger {
        self.params().logger()
    }

    async fn run(self) -> Result<(), crate::Error> {
        crate::run_protocol(self).await
    }
}

#[async_trait]
pub trait NetworkAndProtocolSetup
where
    <<Self as NetworkAndProtocolSetup>::Client as ProvideRuntimeApi<
        <Self as NetworkAndProtocolSetup>::Block,
    >>::Api: JobsApiForGadget<<Self as NetworkAndProtocolSetup>::Block>,
{
    type Network;
    type Protocol;
    type Client: ClientWithApi<Self::Block, Self::Backend>;
    type Block: Block;
    type Backend: Backend<Self::Block>;

    async fn build_jobs_client(
        &self,
    ) -> Result<JobsClient<Self::Block, Self::Backend, Self::Client>, crate::Error> {
        create_client(self.client(), self.logger(), self.pallet_tx()).await
    }

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Block, Self::Backend, Self::Client>,
    ) -> Result<(Self::Network, Self::Protocol), crate::Error>;
    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter>;
    fn logger(&self) -> DebugLogger;
    fn client(&self) -> Self::Client;
}
