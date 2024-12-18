use crate::error::RunnerError;
use gadget_config::GadgetConfiguration;

#[async_trait::async_trait]
pub trait BlueprintConfig: Send + Sync + 'static {
    async fn register(&self, _env: &GadgetConfiguration) -> Result<(), RunnerError> {
        Ok(())
    }
    async fn requires_registration(&self, _env: &GadgetConfiguration) -> Result<bool, RunnerError> {
        Ok(true)
    }
}

impl BlueprintConfig for () {}
