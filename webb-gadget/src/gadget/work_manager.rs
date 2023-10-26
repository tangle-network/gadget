use crate::gadget::message::GadgetProtocolMessage;
use gadget::job_manager::WorkManagerInterface;
use std::sync::Arc;

pub struct ZKWorkManager {
    pub(crate) clock: Arc<dyn Fn() -> ZKWorkManager::Clock>,
}

const ACCEPTABLE_BLOCK_TOLERANCE: u64 = 5;

impl WorkManagerInterface for ZKWorkManager {
    type SSID = u16;
    type Clock = u64;
    type ProtocolMessage = GadgetProtocolMessage;
    type Error = crate::Error;
    type SessionID = u64;

    fn debug(&self, input: String) {
        log::debug!(target: "gadget", "{input}")
    }

    fn error(&self, input: String) {
        log::error!(target: "gadget", "{input}")
    }

    fn warn(&self, input: String) {
        log::warn!(target: "gadget", "{input}")
    }

    fn clock(&self) -> Self::Clock {
        (self.clock)()
    }

    fn acceptable_block_tolerance() -> Self::Clock {
        ACCEPTABLE_BLOCK_TOLERANCE
    }
}
