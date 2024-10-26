use crate::config::GadgetConfiguration;

use super::{BlueprintConfig, RunnerError};

#[derive(Clone, Copy)]
pub struct SymbioticConfig {}

#[async_trait::async_trait]
impl BlueprintConfig for SymbioticConfig {
    async fn register(
        &self,
        _env: &GadgetConfiguration<parking_lot::RawRwLock>,
    ) -> Result<(), RunnerError> {
        Ok(())
    }
}
