
/// Provides access to Eigenlayer utilities through its [`EigenlayerClient`].
#[async_trait::async_trait]
pub trait EigenlayerContext {
    fn client(&self) -> gadget_clients::evm::eigenlayer::EigenlayerClient;
}
