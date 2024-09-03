use async_trait::async_trait;
use round_based::ProtocolMessage;
use serde::{Deserialize, Serialize};
use sp_core::ecdsa;

pub mod channels;
pub mod gossip;
pub mod handlers;
#[cfg(target_family = "wasm")]
pub mod matchbox;

pub mod channels;

pub mod setup;

#[async_trait]
pub trait Network: Send + Sync + Clone + 'static {
    async fn next_message(&self) -> Option<ProtocolMessage>;
    async fn send_message(&self, message: ProtocolMessage) -> Result<(), Error>;

    /// The ID of the king. Only relevant for ZkSaaS networks
    fn greatest_authority_id(&self) -> Option<ecdsa::Public> {
        None
    }
    /// If the network implementation requires a custom runtime, this function
    /// should be manually implemented to keep the network alive
    async fn run(self) -> Result<(), Error> {
        Ok(())
    }
}

pub fn deserialize<'a, T>(data: &'a [u8]) -> Result<T, serde_json::Error>
where
    T: Deserialize<'a>,
{
    serde_json::from_slice::<T>(data)
}

pub fn serialize(object: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(object)
}
