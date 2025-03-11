pub use gadget_clients::tangle::services::TangleServicesClient;

/// `ServicesContext` trait provides access to the Services client from the context.
#[async_trait::async_trait]
pub trait ServicesContext {
    /// Returns the Services client instance
    async fn services_client(
        &self,
    ) -> TangleServicesClient<tangle_subxt::subxt_core::config::PolkadotConfig>;
}
