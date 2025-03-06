pub use gadget_clients::eigenlayer::client::EigenlayerClient;
use gadget_clients::Error;
use blueprint_runner::config::BlueprintEnvironment;

/// Provides access to Eigenlayer utilities through its [`EigenlayerClient`].
#[async_trait::async_trait]
pub trait EigenlayerContext {
    async fn eigenlayer_client(&self) -> Result<EigenlayerClient, Error>;
}

#[async_trait::async_trait]
impl EigenlayerContext for BlueprintEnvironment {
    async fn eigenlayer_client(&self) -> Result<EigenlayerClient, Error> {
        Ok(EigenlayerClient::new(self.clone()))
    }
}
