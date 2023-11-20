use crate::gadget::work_manager::WebbWorkManager;
use gadget_core::job_manager::{ProtocolMessageMetadata, WorkManagerInterface};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct GadgetProtocolMessage {
    pub associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
    pub associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    pub associated_ssid: <WebbWorkManager as WorkManagerInterface>::SSID,
    // A unique marker for the associated task this message belongs to
    pub task_hash: <WebbWorkManager as WorkManagerInterface>::TaskID,
    pub from: UserID,
    // If None, this is a broadcasted message
    pub to: Option<UserID>,
    pub payload: Vec<u8>,
}

pub type UserID = u32;

impl ProtocolMessageMetadata<WebbWorkManager> for GadgetProtocolMessage {
    fn associated_block_id(&self) -> <WebbWorkManager as WorkManagerInterface>::Clock {
        self.associated_block_id
    }

    fn associated_session_id(&self) -> <WebbWorkManager as WorkManagerInterface>::SessionID {
        self.associated_session_id
    }

    fn associated_ssid(&self) -> <WebbWorkManager as WorkManagerInterface>::SSID {
        self.associated_ssid
    }

    fn associated_task(&self) -> <WebbWorkManager as WorkManagerInterface>::TaskID {
        self.task_hash
    }
}
