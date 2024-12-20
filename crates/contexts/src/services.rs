pub use gadget_client_tangle::services::ServicesClient;
use gadget_clients::GadgetServicesClient;
use gadget_std::ops::Deref;
use std::sync::Arc;

/// A blockchain-agnostic client that exposed service-related functionality.
#[derive(Clone)]
pub struct ServicesClient<T: GadgetServicesClient> {
    inner: Arc<T>,
}

impl<T: GadgetServicesClient> Deref for ServicesClient<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

/// `ServicesContext` trait provides access to the Services client from the context
pub trait ServicesContext: Send + Sync + 'static {
    type Client: GadgetServicesClient + Clone;

    /// Returns a ServicesClient used to interact with the Services API
    fn services_client(&self) -> ServicesClient<Self::Client> {
        ServicesClient {
            inner: Arc::new(self.client().clone()),
        }
    }

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
    /// All contexts must have a field containing a services client. Auto-implemented by the macro
    #[doc(hidden)]
    fn client(&self) -> &Self::Client;
}
