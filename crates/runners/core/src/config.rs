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
}

impl BlueprintConfig for () {}
