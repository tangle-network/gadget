pub use gadget_clients::tangle::services::TangleServicesClient;

/// `ServicesContext` trait provides access to the Services client from the context.
#[async_trait::async_trait]
pub trait ServicesContext {
    /// Dynamic injection of the call id at runtime. The code that calls this function
    /// is macro-generated
    #[doc(hidden)]
    fn set_call_id(&mut self, call_id: u64) {
        *self.get_call_id() = Some(call_id);
    }

    /// All contexts that require interaction with the services interface must have a field for storing
    /// a dynamic call ID injected at runtime. Auto-implemented by the macro
    #[doc(hidden)]
    fn get_call_id(&mut self) -> &mut Option<u64>;

    /// Returns the Services client instance
    async fn services_client(
        &self,
    ) -> TangleServicesClient<tangle_subxt::subxt_core::config::PolkadotConfig>;
}
