use crate::gadget::message::GadgetProtocolMessage;
use gadget_core::job_manager::WorkManagerInterface;
use std::sync::Arc;

pub struct WebbWorkManager {
    pub(crate) clock: Arc<
        dyn Fn() -> Option<<WebbWorkManager as WorkManagerInterface>::Clock>
            + Send
            + Sync
            + 'static,
    >,
}

impl WebbWorkManager {
    pub fn new(
        clock: impl Fn() -> Option<<WebbWorkManager as WorkManagerInterface>::Clock>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            clock: Arc::new(clock),
        }
    }
}

const ACCEPTABLE_BLOCK_TOLERANCE: u64 = 5;

impl WorkManagerInterface for WebbWorkManager {
    type SSID = u16;
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
        (self.clock)().expect("No finality notification received")
    }

    fn acceptable_block_tolerance() -> Self::Clock {
        ACCEPTABLE_BLOCK_TOLERANCE
    }
}
