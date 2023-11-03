use crate::error::TestError;
use crate::message::{TestProtocolMessage, UserID};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

// We will use in-memory channels to mock a network
#[derive(Clone)]
pub struct InMemoryNetwork {
    pub to_peers:
        Arc<RwLock<HashMap<UserID, tokio::sync::mpsc::UnboundedSender<TestProtocolMessage>>>>,
}

impl InMemoryNetwork {
    pub fn new(n_peers: usize) -> (Self, Vec<UnboundedReceiver<TestProtocolMessage>>) {
        let mut to_peers = HashMap::new();
        let mut from_peers = Vec::new();
        for i in 0..n_peers {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            to_peers.insert(i as UserID, tx);
            from_peers.push(rx);
        }

        (
            Self {
                to_peers: Arc::new(RwLock::new(to_peers)),
            },
            from_peers,
        )
    }

    pub fn broadcast(&self, msg: TestProtocolMessage) -> Result<(), TestError> {
        let to_peers = self.to_peers.read();
        for (_, tx) in to_peers.iter() {
            tx.send(msg.clone()).map_err(|_| TestError {
                reason: "Failed to broadcast".to_string(),
            })?;
        }

        Ok(())
    }

    pub fn send_to(&self, msg: TestProtocolMessage, to: UserID) -> Result<(), TestError> {
        let to_peers = self.to_peers.read();
        to_peers
            .get(&to)
            .ok_or_else(|| TestError {
                reason: "Peer does not exist in network".to_string(),
            })?
            .send(msg)
            .map_err(|_| TestError {
                reason: "Failed to send to peer".to_string(),
            })?;

        Ok(())
    }
}
