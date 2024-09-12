use gadget_sdk::network::gossip::GossipHandle;

pub mod keygen;
pub mod refresh;
pub mod sign;

pub use keygen::*;
pub use refresh::*;
pub use sign::*;

#[derive(Clone)]
pub struct Context {
    pub network: GossipHandle,
}

impl Context {
    // TODO: Get my index in the network (deterministically)
    // TODO: Get the mapping of all peers to their keys
}
