/// `ServicesContext` trait provides access to the Services client from the context.
pub trait ServicesContext {
    /// Returns the Services client instance
    fn client(&self) -> gadget_client_tangle::services::ServicesClient<subxt::PolkadotConfig>;
}
