use gadget_common::environments::GadgetEnvironment;
use gadget_common::prelude::JobsClient;
use gadget_common::tangle_subxt::subxt::{Config, OnlineClient};

pub type BlockHash = [u8; 32];

/// A client for interacting with the services API
/// TODO: Make this the object that is used for getting services and submitting transactions as necessary.
pub struct ServicesClient<Env: GadgetEnvironment, C: Config> {
    tx_manager: JobsClient<Env>,
    rpc_client: OnlineClient<C>,
}
