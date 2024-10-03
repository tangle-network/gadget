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
