use gadget_clients::tangle::services::ServicesClient;
use subxt::Config;

/// `ServicesContext` trait provides access to the Services client from the context.
pub trait ServicesContext {
    /// Returns the Services client instance
    fn client<C: Config>(&self) -> ServicesClient<C>;
}
