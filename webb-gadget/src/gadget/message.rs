use crate::gadget::work_manager::ZKWorkManager;
use gadget::job_manager::ProtocolMessageMetadata;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GadgetProtocolMessage {
    pub associated_block_id: ZKWorkManager::Clock,
    pub associated_session_id: ZKWorkManager::SessionID,
    pub associated_ssid: ZKWorkManager::SSID,
    pub payload: Vec<u8>,
}

impl ProtocolMessageMetadata<ZKWorkManager> for GadgetProtocolMessage {
    fn associated_block_id(&self) -> ZKWorkManager::Clock {
        self.associated_block_id
    }

    fn associated_session_id(&self) -> ZKWorkManager::SessionID {
        self.associated_session_id
    }

    fn associated_ssid(&self) -> ZKWorkManager::SSID {
        self.associated_ssid
    }
}
