use crate::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_core::job_manager::WorkManagerInterface;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct WebbWorkManager {
    pub(crate) clock: Arc<RwLock<Option<<WebbWorkManager as WorkManagerInterface>::Clock>>>,
}

const ACCEPTABLE_BLOCK_TOLERANCE: u64 = 5;

impl WorkManagerInterface for WebbWorkManager {
    type RetryID = u16;
    type UserID = UserID;
    type Clock = u64;
    type ProtocolMessage = GadgetProtocolMessage;
    type Error = crate::Error;
    type SessionID = u64;
    type TaskID = [u8; 32];

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
        self.clock
            .read()
            .expect("No finality notification received")
    }

    fn acceptable_block_tolerance() -> Self::Clock {
        ACCEPTABLE_BLOCK_TOLERANCE
    }
}
