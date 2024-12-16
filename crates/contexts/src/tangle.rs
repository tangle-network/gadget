/// `TangleContext` trait provides access to the Tangle client from the context.
pub trait TangleClientContext {
    /// Returns the Tangle client instance
    fn client(&self) -> gadget_client_tangle::runtime::TangleClient;

    fn get_call_id(&mut self) -> &mut Option<u64>;

    fn set_call_id(&mut self, call_id: u64) {
        *self.get_call_id() = Some(call_id);
    }
}
