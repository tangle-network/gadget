pub use gadget_clients::tangle::client::TangleClient;
pub use gadget_clients::Error;
use gadget_config::GadgetConfiguration;

/// `TangleContext` trait provides access to the Tangle client from the context.
#[async_trait::async_trait]
pub trait TangleClientContext {
    /// Returns the Tangle client instance
    async fn tangle_client(&self) -> Result<TangleClient, Error>;
}

#[async_trait::async_trait]
impl TangleClientContext for GadgetConfiguration {
    async fn tangle_client(&self) -> Result<TangleClient, Error> {
        TangleClient::new(self.clone()).await.map_err(Into::into)
    }
}
