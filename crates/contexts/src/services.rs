pub use gadget_client_tangle::services::ServicesClient;

/// `ServicesContext` trait provides access to the Services client from the context.
#[async_trait::async_trait]
pub trait ServicesContext {
    /// Returns the Services client instance
    async fn client(&self)
        -> ServicesClient<subxt::PolkadotConfig>;
}
