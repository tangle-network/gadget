use crate::client::ClientWithApi;
use crate::gadget::substrate::TangleEvent;
use crate::prelude::GadgetProtocolMessage;
use tangle_subxt::subxt::{OnlineClient, PolkadotConfig};

pub trait Environment {
    type Event: Send + Sync + 'static;
    type ProtocolMessage: Send + Sync + 'static;
    type Client: ClientWithApi<Self::Event>;
}

#[derive(Default)]
pub struct TangleEnvironment;

impl Environment for TangleEnvironment {
    type Event = TangleEvent;
    type ProtocolMessage = GadgetProtocolMessage;
    type Client = OnlineClient<PolkadotConfig>;
}
