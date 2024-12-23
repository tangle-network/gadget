pub use gadget_client_tangle::runtime::TangleClient;

/// `TangleContext` trait provides access to the Tangle client from the context.
#[async_trait::async_trait]
pub trait TangleClientContext {
    /// Returns the Tangle client instance
    async fn tangle_client(&self) -> Result<TangleClient, subxt::Error>;

    fn get_call_id(&mut self) -> &mut Option<u64>;

    fn set_call_id(&mut self, call_id: u64) {
        *self.get_call_id() = Some(call_id);
    }
}
