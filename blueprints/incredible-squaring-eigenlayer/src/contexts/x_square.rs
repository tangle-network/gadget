use parking_lot::RawRwLock;
use gadget_sdk::config::GadgetConfiguration;
use crate::contexts::client::AggregatorClient;

#[derive(Clone)]
pub struct EigenSquareContext {
    pub client: AggregatorClient,
    pub env: GadgetConfiguration<RawRwLock>
}