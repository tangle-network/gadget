use crate::{gadget::work_manager::TangleWorkManager, utils::serialize};
use gadget_core::job_manager::{ProtocolMessageMetadata, WorkManagerInterface};
use serde::{Deserialize, Serialize};
use sp_core::ecdsa;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TangleProtocolMessage {
    pub associated_block_id: <TangleWorkManager as WorkManagerInterface>::Clock,
    pub associated_session_id: <TangleWorkManager as WorkManagerInterface>::SessionID,
    pub associated_retry_id: <TangleWorkManager as WorkManagerInterface>::RetryID,
    // A unique marker for the associated task this message belongs to
    pub task_hash: <TangleWorkManager as WorkManagerInterface>::TaskID,
    pub from: UserID,
    // If None, this is a broadcasted message
    pub to: Option<UserID>,
    pub payload: Vec<u8>,
    // Some protocols need this additional data
    pub from_network_id: Option<ecdsa::Public>,
    // Some protocol need this additional data
    pub to_network_id: Option<ecdsa::Public>,
}

pub type UserID = u16;

impl ProtocolMessageMetadata<TangleWorkManager> for TangleProtocolMessage {
    fn associated_block_id(&self) -> <TangleWorkManager as WorkManagerInterface>::Clock {
        self.associated_block_id
    }

    fn associated_session_id(&self) -> <TangleWorkManager as WorkManagerInterface>::SessionID {
        self.associated_session_id
    }

    fn associated_retry_id(&self) -> <TangleWorkManager as WorkManagerInterface>::RetryID {
        self.associated_retry_id
    }

    fn associated_task(&self) -> <TangleWorkManager as WorkManagerInterface>::TaskID {
        self.task_hash
    }

    fn associated_sender_user_id(&self) -> u16 {
        self.from
    }

    fn associated_recipient_user_id(&self) -> Option<u16> {
        self.to
    }

    fn payload(&self) -> &Vec<u8> {
        &self.payload
    }

    fn payload_mut(&mut self) -> &mut Vec<u8> {
        &mut self.payload
    }

    fn sender_network_id(&self) -> Option<ecdsa::Public> {
        self.from_network_id
    }

    fn recipient_network_id(&self) -> Option<ecdsa::Public> {
        self.to_network_id
    }
}

impl TangleProtocolMessage {
    /// Returns a hash of the message
    pub fn hash(&self) -> Vec<u8> {
        let serialized = serialize(&self).expect("Should serialize");
        sp_core::keccak_256(&serialized).to_vec()
    }
}
