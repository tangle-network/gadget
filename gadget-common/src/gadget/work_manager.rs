use crate::debug_logger::DebugLogger;
use crate::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_core::job_manager::WorkManagerInterface;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct WebbWorkManager {
    pub(crate) clock: Arc<RwLock<Option<<WebbWorkManager as WorkManagerInterface>::Clock>>>,
    pub(crate) logger: DebugLogger,
}

/// Acceptable delivery tolerance for a message. Messages that fall out
/// of this range will be rejected by the work manager
const ACCEPTABLE_BLOCK_TOLERANCE: u64 = 10;

impl WorkManagerInterface for WebbWorkManager {
    type RetryID = u16;
    type UserID = UserID;
    type Clock = u64;
    type ProtocolMessage = GadgetProtocolMessage;
    type Error = crate::Error;
    type SessionID = u64;
    type TaskID = [u8; 32];

    fn debug(&self, input: String) {
        self.logger.debug(input)
    }

    fn error(&self, input: String) {
        self.logger.error(input)
    }

    fn warn(&self, input: String) {
        self.logger.warn(input)
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
