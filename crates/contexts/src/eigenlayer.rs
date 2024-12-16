/// Provides access to Eigenlayer utilities through its [`EigenlayerClient`].
pub trait EigenlayerContext {
    fn client(&self) -> gadget_client_eigenlayer::eigenlayer::EigenlayerClient;
}
