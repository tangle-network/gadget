use crate::config::StdGadgetConfiguration;

/// The interface for running a gadget.
///
/// This trait provides methods for managing the lifecycle of a gadget,
/// including registration, benchmarking, and execution.
#[async_trait::async_trait]
pub trait GadgetRunner {
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
    /// # Returns
    ///
    /// Returns an error if the benchmarking fails.
    async fn benchmark(&self) -> Result<(), Self::Error>;

    /// Executes the gadget's main functionality.
    ///
    /// # Returns
    ///
    /// Returns an error if the gadget execution fails.
    async fn run(&mut self) -> Result<(), Self::Error>;
}
