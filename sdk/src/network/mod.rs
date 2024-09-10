use core::fmt::Display;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sp_core::ecdsa;

use crate::error::Error;

use self::channels::UserID;

pub mod channels;
pub mod gossip;
pub mod handlers;
#[cfg(target_family = "wasm")]
pub mod matchbox;
pub mod setup;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct IdentifierInfo {
    pub block_id: Option<u64>,
    pub session_id: Option<u64>,
    pub retry_id: Option<u64>,
    pub task_id: Option<u64>,
}

impl Display for IdentifierInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let block_id = self
            .block_id
            .map(|id| format!("block_id: {}", id))
            .unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| format!("session_id: {}", id))
            .unwrap_or_default();
        let retry_id = self
            .retry_id
            .map(|id| format!("retry_id: {}", id))
            .unwrap_or_default();
        let task_id = self
            .task_id
            .map(|id| format!("task_id: {}", id))
            .unwrap_or_default();
        write!(f, "{} {} {} {}", block_id, session_id, retry_id, task_id)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ParticipantInfo {
    pub user_id: u16,
    pub ecdsa_key: Option<sp_core::ecdsa::Public>,
}

impl Display for ParticipantInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ecdsa_key = self
            .ecdsa_key
            .map(|key| format!("ecdsa_key: {}", key))
            .unwrap_or_default();
        write!(f, "user_id: {}, {}", self.user_id, ecdsa_key)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolMessage {
    pub identifier_info: IdentifierInfo,
    pub sender: ParticipantInfo,
    pub recipient: Option<ParticipantInfo>,
    pub payload: Vec<u8>,
}

impl Display for ProtocolMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "identifier_info: {}, sender: {}, recipient: {:?}, payload: {:?}",
            self.identifier_info, self.sender, self.recipient, self.payload
        )
    }
}

#[async_trait]
pub trait Network: Send + Sync + Clone + 'static {
    async fn next_message(&self) -> Option<ProtocolMessage>;
    async fn send_message(&self, message: ProtocolMessage) -> Result<(), Error>;

    /// If the network implementation requires a custom runtime, this function
    /// should be manually implemented to keep the network alive
    async fn run(self) -> Result<(), Error> {
        Ok(())
    }

    fn build_protocol_message<Payload: Serialize>(
        identifier_info: IdentifierInfo,
        from: UserID,
        to: Option<UserID>,
        payload: &Payload,
        from_account_id: Option<ecdsa::Public>,
        to_network_id: Option<ecdsa::Public>,
    ) -> ProtocolMessage {
        let sender_participant_info = ParticipantInfo {
            user_id: from,
            ecdsa_key: from_account_id,
        };
        let receiver_participant_info = to.map(|to| ParticipantInfo {
            user_id: to,
            ecdsa_key: to_network_id,
        });
        ProtocolMessage {
            identifier_info,
            sender: sender_participant_info,
            recipient: receiver_participant_info,
            payload: serialize(payload).expect("Failed to serialize message"),
        }
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
