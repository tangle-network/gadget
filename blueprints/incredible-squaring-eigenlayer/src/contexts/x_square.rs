use crate::contexts::client::AggregatorClient;
use gadget_sdk::{config::StdGadgetConfiguration, contexts::KeystoreContext};

#[derive(Clone, KeystoreContext)]
pub struct EigenSquareContext {
    pub client: AggregatorClient,
    #[config]
    pub std_config: StdGadgetConfiguration,
}
