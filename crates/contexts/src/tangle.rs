/// `TangleContext` trait provides access to the Tangle client from the context.
pub trait TangleContext {
    /// Returns the Tangle client instance
    fn client(&self) -> gadget_client_tangle::runtime::TangleClient;
}
