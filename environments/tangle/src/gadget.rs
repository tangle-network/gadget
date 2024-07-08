use gadget_common::{
    tangle_runtime::{AccountId32, MaxAdditionalParamsLen, MaxParticipants, MaxSubmissionLen},
    tangle_subxt::tangle_testnet_runtime::api::runtime_types::{
        sp_core::ecdsa,
        tangle_primitives::{jobs, roles},
    },
    WorkManagerInterface,
};
use gadget_core::gadget::substrate::FinalityNotification;

use crate::work_manager::TangleWorkManager;

pub type TangleEvent = FinalityNotification;

pub struct TangleJobMetadata {
    pub job_type:
        jobs::JobType<AccountId32, MaxParticipants, MaxSubmissionLen, MaxAdditionalParamsLen>,
    pub role_type: roles::RoleType,
    /// This value only exists if this is a stage2 job
    pub phase1_job: Option<
        jobs::JobType<AccountId32, MaxParticipants, MaxSubmissionLen, MaxAdditionalParamsLen>,
    >,
    pub participants_role_ids: Vec<ecdsa::Public>,
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
