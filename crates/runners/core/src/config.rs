use crate::error::RunnerError;
use gadget_config::GadgetConfiguration;

pub trait CloneableConfig: Send + Sync {
    fn clone_box(&self) -> Box<dyn BlueprintConfig>;
}

impl<T> CloneableConfig for T
where
    T: BlueprintConfig + Clone,
{
    fn clone_box(&self) -> Box<dyn BlueprintConfig> {
        Box::new(self.clone())
    }
}

#[async_trait::async_trait]
pub trait BlueprintConfig: Send + Sync + CloneableConfig + 'static {
    async fn register(&self, _env: &GadgetConfiguration) -> Result<(), RunnerError> {
        Ok(())
    }

    async fn requires_registration(&self, _env: &GadgetConfiguration) -> Result<bool, RunnerError> {
        Ok(true)
    }
    /// Controls whether the runner should exit after registration
    ///
    /// Returns true if the runner should exit after registration, false if it should continue
    fn should_exit_after_registration(&self) -> bool {
        true // By default, runners exit after registration
    }
}

impl BlueprintConfig for () {}
