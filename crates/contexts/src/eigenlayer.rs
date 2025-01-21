pub use gadget_clients::eigenlayer::client::EigenlayerClient;
use gadget_clients::Error;
use gadget_config::GadgetConfiguration;

/// Provides access to Eigenlayer utilities through its [`EigenlayerClient`].
#[async_trait::async_trait]
pub trait EigenlayerContext {
    async fn eigenlayer_client(&self) -> Result<EigenlayerClient, Error>;
}

#[async_trait::async_trait]
impl EigenlayerContext for GadgetConfiguration {
    async fn eigenlayer_client(&self) -> Result<EigenlayerClient, Error> {
        Ok(EigenlayerClient::new(self.clone()))
    }
}
