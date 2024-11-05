use crate::contexts::client::AggregatorClient;
use gadget_sdk::{config::StdGadgetConfiguration, ctx::KeystoreContext};

#[derive(Clone, KeystoreContext)]
pub struct EigenSquareContext {
    pub client: AggregatorClient,
    #[config]
    pub std_config: StdGadgetConfiguration,
}
