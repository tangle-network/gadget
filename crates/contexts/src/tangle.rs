use gadget_clients::tangle::runtime::TangleClient;
use std::future::Future;

/// `TangleContext` trait provides access to the Tangle client from the context.
pub trait TangleContext {
    /// Returns the Tangle client instance
    fn client(&self) -> TangleClient;
}
