pub use crate::client::ClientWithApi;
use crate::client::{create_client, JobsClient, PalletSubmitter};
pub use crate::debug_logger::DebugLogger;
pub use crate::gadget::network::Network;
pub use crate::gadget::GadgetProtocol;
use async_trait::async_trait;
pub use gadget_io::PrometheusConfig;
use std::sync::Arc;

#[async_trait]
pub trait ProtocolConfig
where
    Self: Sized,
{
    type Network: Network;
    type Protocol: GadgetProtocol<
        <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Client,
    >;
    type ProtocolSpecificConfiguration: Clone
        + NetworkAndProtocolSetup<Network = Self::Network, Protocol = Self::Protocol>
        + Sync;
    fn params(&self) -> &Self::ProtocolSpecificConfiguration;

    fn take_network(&mut self) -> Self::Network;
    fn take_protocol(&mut self) -> Self::Protocol;
    fn take_client(
        &mut self,
    ) -> <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Client;
    fn prometheus_config(&self) -> PrometheusConfig;

    async fn build(&self) -> Result<Self, gadget_io::Error> {
        let jobs_client = self.params().build_jobs_client().await?;
        let (network, protocol) = self
            .params()
            .build_network_and_protocol(jobs_client)
            .await?;
        let client = self.params().client();
        let params = self.params().clone();
        let pallet_tx = self.pallet_tx();
        let logger = self.logger();
        let prometheus_config = self.prometheus_config();

        Ok(Self::new(
            network,
            client,
            protocol,
            params,
            pallet_tx,
            logger,
            prometheus_config,
        ))
    }

    fn new(
        network: Self::Network,
        client: <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup>::Client,
        protocol: Self::Protocol,
        params: Self::ProtocolSpecificConfiguration,
        pallet_tx: Arc<dyn PalletSubmitter>,
        logger: DebugLogger,
        prometheus_config: PrometheusConfig,
    ) -> Self;

    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter> {
        self.params().pallet_tx()
    }
    fn logger(&self) -> DebugLogger {
        self.params().logger()
    }

    async fn run(self) -> Result<(), gadget_io::Error> {
        crate::run_protocol(self).await
    }
}

#[async_trait]
pub trait NetworkAndProtocolSetup {
    type Network;
    type Protocol;
    type Client: ClientWithApi;

    async fn build_jobs_client(&self) -> Result<JobsClient<Self::Client>, gadget_io::Error> {
        create_client(self.client(), self.logger(), self.pallet_tx()).await
    }

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Client>,
    ) -> Result<(Self::Network, Self::Protocol), gadget_io::Error>;
    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter>;
    fn logger(&self) -> DebugLogger;
    fn client(&self) -> Self::Client;
}
