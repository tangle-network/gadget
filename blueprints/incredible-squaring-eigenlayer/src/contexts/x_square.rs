use crate::contexts::client::AggregatorClient;
use gadget_config::GadgetConfiguration;
use gadget_macros::contexts::KeystoreContext;

#[derive(Clone, KeystoreContext)]
pub struct EigenSquareContext {
    pub client: AggregatorClient,
    #[config]
    pub std_config: GadgetConfiguration,
}
