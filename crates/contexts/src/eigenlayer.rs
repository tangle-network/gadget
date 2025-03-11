use blueprint_runner::config::BlueprintEnvironment;
use gadget_clients::Error;
pub use gadget_clients::eigenlayer::client::EigenlayerClient;

/// Provides access to Eigenlayer utilities through its [`EigenlayerClient`].
pub trait EigenlayerContext {
    async fn eigenlayer_client(&self) -> Result<EigenlayerClient, Error>;
}

impl EigenlayerContext for BlueprintEnvironment {
    async fn eigenlayer_client(&self) -> Result<EigenlayerClient, Error> {
        Ok(EigenlayerClient::new(self.clone()))
    }
}
