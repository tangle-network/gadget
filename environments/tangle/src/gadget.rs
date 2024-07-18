use crate::work_manager::TangleWorkManager;
use gadget_common::WorkManagerInterface;
use gadget_core::gadget::substrate::FinalityNotification;

pub type TangleEvent = FinalityNotification;

pub struct TangleJobMetadata {
    pub task_id: <TangleWorkManager as WorkManagerInterface>::TaskID,
    pub retry_id: <TangleWorkManager as WorkManagerInterface>::RetryID,
    pub job_id: u64,
    pub now: <TangleWorkManager as WorkManagerInterface>::Clock,
    pub at: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct SubxtConfig {
    /// The URL of the Tangle Node.
    pub endpoint: url::Url,
}
