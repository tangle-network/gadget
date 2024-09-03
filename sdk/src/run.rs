use crate::config::StdGadgetConfiguration;

/// A trait defining the interface for running a gadget.
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
    /// A reference to the `StdGadgetConfiguration` for this gadget.
    fn config(&self) -> &StdGadgetConfiguration;

    /// Registers the current operator with the the blueprint.
    ///
    /// This method should be called only during the registration mode.
    ///
    /// # Returns
    /// A `Result` indicating success or an error if registration fails.
    async fn register(&self) -> Result<(), Self::Error>;

    /// Performs a benchmark of the gadget's performance.
    ///
    /// # Returns
    /// A `Result` indicating success or an error if benchmarking fails.
    async fn benchmark(&self) -> Result<(), Self::Error>;

    /// Executes the gadget's main functionality.
    ///
    /// # Returns
    /// A `Result` indicating success or an error if the execution fails.
    async fn run(&self) -> Result<(), Self::Error>;
}
