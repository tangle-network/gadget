use parking_lot::RwLock;

use crate::{
    config::{ContextConfig, GadgetCLICoreSettings, GadgetConfiguration, StdGadgetConfiguration},
    events_watcher::{evm, substrate},
    info,
};

pub mod eigenlayer;
pub mod tangle;

/// The interface for running a gadget.
///
/// This trait provides methods for managing the lifecycle of a gadget,
/// including registration, benchmarking, and execution.
#[async_trait::async_trait]
pub trait GadgetRunner: Sized + Send + Sync {
    /// The type of error that can be returned by the gadget runner methods.
    type Error;

    /// Returns a reference to the gadget's standard configuration.
    ///
    /// # Returns
    ///
    /// A reference to the `StdGadgetConfiguration` for this gadget.
    fn config(&self) -> &StdGadgetConfiguration;

    /// Registers the current operator with the blueprint.
    ///
    /// This method should be called only during the registration mode.
    ///
    /// # Errors
    ///
    /// Returns an error if the registration fails.
    async fn register(&mut self) -> Result<(), Self::Error>;

    /// Performs a benchmark of the gadget's performance.
    ///
    /// # Errors
    ///
    /// Returns an error if the benchmarking fails.
    async fn benchmark(&self) -> Result<(), Self::Error>;

    /// Executes the gadget's main functionality.
    ///
    /// # Errors
    ///
    /// Returns an error if the gadget execution fails.
    async fn run(&self) -> Result<(), Self::Error>;

    /// Creates a new instance of the gadget runner.
    ///
    /// # Returns
    ///
    /// A new instance of the gadget runner.
    ///
    /// # Errors
    ///
    /// Returns an error if the gadget runner cannot be created.
    async fn create(config: ContextConfig) -> Result<Self, Self::Error>;

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

    /// Registers a Substrate event handler.
    ///
    /// # Parameters
    ///
    /// * `handler` - The Substrate event handler to register.
    ///
    /// # Note
    ///
    /// This method is used to register a Substrate event handler with the gadget runner.
    fn register_substrate_event_handler<F>(&self, handler: F)
    where
        F: Fn(&dyn substrate::EventHandler) + Send + Sync + 'static,
    {
        if let Some(handlers) = self.substrate_event_handlers() {
            handlers.write().unwrap().push(Box::new(handler));
        }
    }

    /// Retrieves all Substrate event handlers.
    ///
    /// # Returns
    ///
    /// A vector of Substrate event handlers.
    ///
    /// # Note
    ///
    /// This method is used to retrieve all Substrate event handlers that have been registered with the gadget runner.
    fn get_substrate_event_handlers(&self) -> Vec<dyn substrate::EventHandler> {
        self.substrate_event_handlers()
            .map(|handlers| handlers.read().unwrap().clone())
            .unwrap_or_default()
    }

    /// Returns a reference to the Substrate event handlers storage.
    /// This method should be implemented by the struct to provide access to its Substrate event handlers.
    ///
    /// # Returns
    ///
    /// A reference to the Substrate event handlers storage.
    ///
    /// # Note
    ///
    /// This method is used to provide access to the Substrate event handlers storage for the gadget runner.
    fn substrate_event_handlers(&self) -> Option<&RwLock<Vec<dyn substrate::EventHandler>>>;
}

#[allow(irrefutable_let_patterns)]
pub fn check_for_test(
    _env: &GadgetConfiguration<parking_lot::RawRwLock>,
    config: &ContextConfig,
) -> Result<(), String> {
    // create a file to denote we have started
    if let GadgetCLICoreSettings::Run {
        keystore_uri: base_path,
        test_mode,
        ..
    } = &config.gadget_core_settings
    {
        if !*test_mode {
            return Ok(());
        }
        let path = base_path.sanitize_file_path().join("test_started.tmp");
        let mut file = std::fs::File::create(&path)?;
        file.write_all(b"test_started")?;
        info!("Successfully wrote test file to {}", path.display())
    }

    Ok(())
}
