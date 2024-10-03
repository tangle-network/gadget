//! Runner for Eigenlayer

trait EvmGadgetRunner: GadgetRunner {
    /// Registers an event handler.
    ///
    /// # Parameters
    ///
    /// * `handler` - The event handler to register.
    ///
    /// # Note
    ///
    /// This method is used to register an event handler with the gadget runner.
    fn register_event_handler<F, T: evm::Config>(&self, handler: F)
    where
        F: Fn(&dyn evm::EventHandler<T>) + Send + Sync + 'static,
    {
        if let Some(handlers) = self.event_handlers() {
            handlers.write().unwrap().push(Box::new(handler));
        }
    }

    /// Retrieves all EVM event handlers.
    ///
    /// # Returns
    ///
    /// A vector of EVM event handlers.
    ///
    /// # Note
    ///
    /// This method is used to retrieve all EVM event handlers that have been registered with the gadget runner.
    fn get_evm_event_handlers<T: evm::Config>(&self) -> Vec<dyn evm::EventHandler<T>> {
        self.event_handlers()
            .map(|handlers| handlers.read().unwrap().clone())
            .unwrap_or_default()
    }

    /// Returns a reference to the EVM event handlers storage.
    /// This method should be implemented by the struct to provide access to its event handlers.
    ///
    /// # Returns
    ///
    /// A reference to the event handlers storage.
    ///
    /// # Note
    ///
    /// This method is used to provide access to the event handlers storage for the gadget runner.
    fn evm_event_handlers<T: evm::Config>(&self) -> Option<&RwLock<Vec<dyn evm::EventHandle<T>>>>;
}