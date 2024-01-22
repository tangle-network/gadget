use crate::client::{create_client, JobsClient, PalletSubmitter};
pub use crate::client::{AccountId, ClientWithApi};
pub use crate::debug_logger::DebugLogger;
pub use crate::gadget::network::Network;
pub use crate::gadget::WebbGadgetProtocol;
use async_trait::async_trait;
pub use pallet_jobs_rpc_runtime_api::JobsApi;
pub use sc_client_api::Backend;
pub use sp_api::ProvideRuntimeApi;
pub use sp_runtime::traits::Block;
use std::sync::Arc;

#[async_trait]
pub trait ProtocolConfig
where
    <Self::Client as ProvideRuntimeApi<Self::Block>>::Api: JobsApi<Self::Block, AccountId>,
    Self: Sized,
{
    type Network: Network;
    type Block: Block;
    type Backend: Backend<Self::Block>;
    type Protocol: WebbGadgetProtocol<Self::Block, Self::Backend, Self::Client>;
    type Client: ClientWithApi<Self::Block, Self::Backend>;
    type ProtocolSpecificConfiguration: Clone
        + NetworkAndProtocolSetup<Network = Self::Network, Protocol = Self::Protocol>;
    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter>;
    fn logger(&self) -> DebugLogger;
    fn client_inner(&self) -> Self::Client;
    fn params(&self) -> &Self::ProtocolSpecificConfiguration;
    async fn build_jobs_client(
        &self,
    ) -> Result<JobsClient<Self::Block, Self::Backend, Self::Client>, crate::Error> {
        create_client(self.client_inner(), self.logger(), self.pallet_tx()).await
    }
    async fn build(&self) -> Result<Self, crate::Error> {
        let (network, protocol) = self.params().build_network_and_protocol().await?;
        let client = self.client_inner();
        let params = self.params().clone();
        let pallet_tx = self.pallet_tx();
        let logger = self.logger();
        Ok(Self::new(
            network, client, protocol, params, pallet_tx, logger,
        ))
    }

    fn new(
        network: Self::Network,
        client: Self::Client,
        protocol: Self::Protocol,
        params: Self::ProtocolSpecificConfiguration,
        pallet_tx: Arc<dyn PalletSubmitter>,
        logger: DebugLogger,
    ) -> Self;

    fn take_network(&mut self) -> Self::Network;
    fn take_protocol(&mut self) -> Self::Protocol;
    fn take_client(&mut self) -> Self::Client;
}

#[async_trait]
pub trait NetworkAndProtocolSetup {
    type Network;
    type Protocol;
    async fn build_network_and_protocol(
        &self,
    ) -> Result<(Self::Network, Self::Protocol), crate::Error>;
    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter>;
    fn logger(&self) -> DebugLogger;
}
