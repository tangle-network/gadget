pub use crate::client::ClientWithApi;
use crate::client::{create_client, JobsClient, PalletSubmitter};
pub use crate::debug_logger::DebugLogger;
use crate::environments::GadgetEnvironment;
pub use crate::gadget::network::Network;
pub use crate::gadget::GadgetProtocol;
pub use crate::prometheus::PrometheusConfig;
use async_trait::async_trait;
use gadget_core::gadget::manager::AbstractGadget;
use std::sync::Arc;

#[async_trait]
pub trait ProtocolConfig<
    Env: GadgetEnvironment,
    Event: Send + Sync + 'static,
    ProtocolMessage: Send + Sync + 'static,
> where
    Self: Sized,
{
    type Network: Network<ProtocolMessage, Event>;
    type Protocol: GadgetProtocol<
        Env,
        <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup<Event, ProtocolMessage>>::Client,
    >;
    type ProtocolSpecificConfiguration: Clone
        + NetworkAndProtocolSetup<Event, ProtocolMessage>
        + Sync;
    fn params(&self) -> &Self::ProtocolSpecificConfiguration;

    fn take_network(&mut self) -> Self::Network;
    fn take_protocol(&mut self) -> Self::Protocol;
    fn take_client(
        &mut self,
    ) -> <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup<Event, ProtocolMessage>>::Client;
    fn prometheus_config(&self) -> PrometheusConfig;

    async fn build(&self) -> Result<Self, crate::Error> {
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
        network: <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup<
            Event,
            ProtocolMessage,
        >>::Network,
        client: <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup<
            Event,
            ProtocolMessage,
        >>::Client,
        protocol: <<Self as ProtocolConfig<Env, Event, ProtocolMessage>>::ProtocolSpecificConfiguration as NetworkAndProtocolSetup<Event, ProtocolMessage>>::Protocol,
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

    async fn run(self) -> Result<(), crate::Error> {
        crate::run_protocol(self).await
    }
}

#[async_trait]
pub trait NetworkAndProtocolSetup<Event: Send + Sync + 'static, ProtocolMessage> {
    type Network;
    type Protocol;
    type Client: ClientWithApi<Event>;

    async fn build_jobs_client(&self) -> Result<JobsClient<Self::Client, Event>, crate::Error> {
        create_client(self.client(), self.logger(), self.pallet_tx()).await
    }

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Self::Client, Event>,
    ) -> Result<(Self::Network, Self::Protocol), crate::Error>;
    fn pallet_tx(&self) -> Arc<dyn PalletSubmitter>;
    fn logger(&self) -> DebugLogger;
    fn client(&self) -> Self::Client;
}
