use crate::keystore::KeystoreUriSanitizer;
use crate::{
    config::{ContextConfig, GadgetCLICoreSettings, GadgetConfiguration, StdGadgetConfiguration},
    events_watcher::{evm, substrate},
    info,
};
use ark_serialize::Write;
use parking_lot::RwLock;
use std::any::Any;

pub mod eigenlayer;
pub mod tangle;

/// The interface for running a gadget.
///
/// This trait provides methods for managing the lifecycle of a gadget,
/// including registration, benchmarking, and execution.
#[async_trait::async_trait]
pub trait GadgetRunner: Sized + Send + Sync {
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
    async fn register(&mut self) -> Result<(), crate::Error>;

    /// Performs a benchmark of the gadget's performance.
    ///
    /// # Errors
    ///
    /// Returns an error if the benchmarking fails.
    async fn benchmark(&self) -> Result<(), crate::Error>;

    /// Executes the gadget's main functionality.
    ///
    /// # Errors
    ///
    /// Returns an error if the gadget execution fails.
    async fn run(&self) -> Result<(), crate::Error>;
}
