use crate::contexts::client::AggregatorClient;
use gadget_sdk::config::GadgetConfiguration;
use parking_lot::RawRwLock;

#[derive(Clone)]
pub struct EigenSquareContext {
    pub client: AggregatorClient,
    pub env: GadgetConfiguration<RawRwLock>,
}
