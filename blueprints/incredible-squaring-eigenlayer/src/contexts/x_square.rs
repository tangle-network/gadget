use crate::contexts::client::AggregatorClient;
use blueprint_sdk::runner::config::BlueprintEnvironment;
use blueprint_sdk::macros::contexts::KeystoreContext;

#[derive(Clone, KeystoreContext)]
pub struct EigenSquareContext {
    pub client: AggregatorClient,
    #[config]
    pub std_config: BlueprintEnvironment,
}
