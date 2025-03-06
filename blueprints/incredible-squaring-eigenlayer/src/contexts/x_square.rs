use crate::contexts::client::AggregatorClient;
use blueprint_sdk::macros::contexts::KeystoreContext;
use blueprint_sdk::runner::config::BlueprintEnvironment;

#[derive(Clone, KeystoreContext)]
pub struct EigenSquareContext {
    pub client: AggregatorClient,
    #[config]
    pub std_config: BlueprintEnvironment,
}
