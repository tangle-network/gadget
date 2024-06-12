use crate::client::{create_client, JobsClient};
pub use crate::debug_logger::DebugLogger;
use crate::environments::GadgetEnvironment;
pub use crate::module::network::Network;
pub use crate::module::GadgetProtocol;
pub use crate::prometheus::PrometheusConfig;
use async_trait::async_trait;
use gadget_core::gadget::general::Client;

#[async_trait]
pub trait ProtocolConfig<Env: GadgetEnvironment>
where
    Self: Sized,
    <Env as GadgetEnvironment>::Client: Client<<Env as GadgetEnvironment>::Event>,
{
    type Network: Network<Env>;
    type Protocol: GadgetProtocol<Env>;
    type ProtocolSpecificConfiguration: Clone + NetworkAndProtocolSetup<Env> + Sync;
    fn params(&self) -> &Self::ProtocolSpecificConfiguration;

    fn take_network(&mut self) -> Self::Network;
    fn take_protocol(&mut self) -> Self::Protocol;
    fn take_client(&mut self) -> <Env as GadgetEnvironment>::Client;
    fn prometheus_config(&self) -> PrometheusConfig;

    async fn build(&self) -> Result<Self, crate::Error> {
        let jobs_client = self.params().build_jobs_client().await?;
        let (network, protocol) = self
            .params()
            .build_network_and_protocol(jobs_client)
            .await?;
        let client = self.params().client();
        let params = self.params().clone();
        let tx_manager = self.tx_manager();
        let logger = self.logger();
        let prometheus_config = self.prometheus_config();

        Ok(Self::new(
            network,
            client,
            protocol,
            params,
            tx_manager,
            logger,
            prometheus_config,
        ))
    }

    fn new(
        network: <Self::ProtocolSpecificConfiguration as NetworkAndProtocolSetup<Env>>::Network,
        client: <Env as GadgetEnvironment>::Client,
        protocol: <<Self as ProtocolConfig<Env>>::ProtocolSpecificConfiguration as NetworkAndProtocolSetup<Env>>::Protocol,
        params: Self::ProtocolSpecificConfiguration,
        tx_manager: <Env as GadgetEnvironment>::TransactionManager,
        logger: DebugLogger,
        prometheus_config: PrometheusConfig,
    ) -> Self;

    fn tx_manager(&self) -> <Env as GadgetEnvironment>::TransactionManager {
        self.params().tx_manager()
    }
    fn logger(&self) -> DebugLogger {
        self.params().logger()
    }

    async fn run(self) -> Result<(), crate::Error> {
        crate::run_protocol(self).await
    }
}

#[async_trait]
pub trait NetworkAndProtocolSetup<Env: GadgetEnvironment> {
    type Network;
    type Protocol;

    async fn build_jobs_client(&self) -> Result<JobsClient<Env>, crate::Error> {
        create_client(self.client(), self.logger(), self.tx_manager()).await
    }

    async fn build_network_and_protocol(
        &self,
        jobs_client: JobsClient<Env>,
    ) -> Result<(Self::Network, Self::Protocol), crate::Error>;
    fn tx_manager(&self) -> <Env as GadgetEnvironment>::TransactionManager;
    fn logger(&self) -> DebugLogger;
    fn client(&self) -> <Env as GadgetEnvironment>::Client;
}
