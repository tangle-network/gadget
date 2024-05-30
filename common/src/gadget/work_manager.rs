use crate::debug_logger::DebugLogger;
use crate::gadget::message::TangleProtocolMessage;
use gadget_core::job_manager::WorkManagerInterface;
use parking_lot::RwLock;
use std::sync::Arc;

pub struct TangleWorkManager {
    pub(crate) clock: Arc<RwLock<Option<<TangleWorkManager as WorkManagerInterface>::Clock>>>,
    pub(crate) logger: DebugLogger,
}

/// Acceptable delivery tolerance for a message. Messages that fall out
/// of this range will be rejected by the work manager
#[cfg(not(feature = "testing"))]
const ACCEPTABLE_BLOCK_TOLERANCE: u64 = 20;
#[cfg(feature = "testing")]
const ACCEPTABLE_BLOCK_TOLERANCE: u64 = 60;

impl WorkManagerInterface for TangleWorkManager {
    type RetryID = u16;
    type Clock = u64;
    type ProtocolMessage = TangleProtocolMessage;
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
