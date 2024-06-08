/*use gadget_common::client::{ClientWithApi, JobsClient, JobTypeExt, PhaseResultExt};
use gadget_common::sp_core::{ByteArray, ecdsa, sr25519};
use gadget_common::tangle_runtime;
use gadget_common::tangle_runtime::{AccountId32, DKGTSSPhaseFour, DKGTSSPhaseOne, DKGTSSPhaseThree, DKGTSSPhaseTwo, jobs, MaxAdditionalParamsLen, MaxDataLen, MaxKeyLen, MaxParticipants, MaxProofLen, MaxSignatureLen, MaxSubmissionLen, RoleType, ZkSaaSPhaseOne, ZkSaaSPhaseTwo};

use crate::TangleEnvironment;

impl JobsClient<TangleEnvironment> {
    pub async fn query_jobs_by_validator(
        &self,
        at: [u8; 32],
        validator: sr25519::Public,
    ) -> Result<
        Vec<
            jobs::RpcResponseJobsData<
                AccountId32,
                u64,
                MaxParticipants,
                MaxSubmissionLen,
                MaxAdditionalParamsLen,
            >,
        >,
        gadget_common::Error,
    > {
        Ok(self
            .client
            .query_jobs_by_validator(at, AccountId32::from(validator.0))
            .await?
            .into_iter()
            .flatten()
            .collect())
    }

    pub async fn query_restaker_role_key(
        &self,
        at: [u8; 32],
        address: sr25519::Public,
    ) -> Result<Option<ecdsa::Public>, gadget_common::Error> {
        self.client
            .query_restaker_role_key(at, AccountId32::from(address.0))
            .await
            .map(|r| r.and_then(|v| ecdsa::Public::from_slice(v.as_slice()).ok()))
    }

    pub async fn query_job_by_id(
        &self,
        at: [u8; 32],
        role_type: tangle_runtime::RoleType,
        job_id: u64,
    ) -> Result<
        Option<
            jobs::RpcResponseJobsData<
                AccountId32,
                u64,
                MaxParticipants,
                MaxSubmissionLen,
                MaxAdditionalParamsLen,
            >,
        >,
        gadget_common::Error,
    > {
        self.client.query_job_by_id(at, role_type, job_id).await
    }

    pub async fn query_job_result(
        &self,
        at: [u8; 32],
        role_type: tangle_runtime::RoleType,
        job_id: u64,
    ) -> Result<
        Option<
            jobs::PhaseResult<
                AccountId32,
                u64,
                MaxParticipants,
                MaxKeyLen,
                MaxDataLen,
                MaxSignatureLen,
                MaxSubmissionLen,
                MaxProofLen,
                MaxAdditionalParamsLen,
            >,
        >,
        gadget_common::Error,
    > {
        self.client.query_job_result(at, role_type, job_id).await
    }

    pub async fn submit_job_result(
        &self,
        role_type: tangle_runtime::RoleType,
        job_id: u64,
        result: jobs::JobResult<
            MaxParticipants,
            MaxKeyLen,
            MaxSignatureLen,
            MaxDataLen,
            MaxProofLen,
            MaxAdditionalParamsLen,
        >,
    ) -> Result<(), gadget_common::Error> {
        self.pallet_tx
            .submit_job_result(role_type, job_id, result)
            .await
    }
}

impl<MaxParticipants, MaxSubmissionLen, MaxAdditionalParamsLen> JobTypeExt
for jobs::JobType<AccountId32, MaxParticipants, MaxSubmissionLen, MaxAdditionalParamsLen>
{
    fn is_phase_one(&self) -> bool {
        use gadget_common::tangle_runtime::jobs::JobType::*;
        matches!(self, DKGTSSPhaseOne(_) | ZkSaaSPhaseOne(_))
    }

    fn get_participants(self) -> Option<Vec<AccountId32>> {
        use gadget_common::tangle_runtime::jobs::JobType::*;
        match self {
            DKGTSSPhaseOne(p) => Some(p.participants.0),
            ZkSaaSPhaseOne(p) => Some(p.participants.0),
            _ => None,
        }
    }

    fn get_threshold(self) -> Option<u8> {
        use gadget_common::tangle_runtime::jobs::JobType::*;
        match self {
            DKGTSSPhaseOne(p) => Some(p.threshold),
            _ => None,
        }
    }

    fn get_role_type(&self) -> tangle_runtime::RoleType {
        match self {
            DKGTSSPhaseOne(job) => RoleType::Tss(job.role_type.clone()),
            ZkSaaSPhaseOne(job) => RoleType::ZkSaaS(job.role_type.clone()),
            DKGTSSPhaseTwo(job) => RoleType::Tss(job.role_type.clone()),
            ZkSaaSPhaseTwo(job) => RoleType::ZkSaaS(job.role_type.clone()),
            DKGTSSPhaseThree(job) => RoleType::Tss(job.role_type.clone()),
            DKGTSSPhaseFour(job) => RoleType::Tss(job.role_type.clone()),
        }
    }

    fn get_phase_one_id(&self) -> Option<u64> {
        use gadget_common::tangle_runtime::jobs::JobType::*;
        match self {
            DKGTSSPhaseTwo(info) => Some(info.phase_one_id),
            DKGTSSPhaseThree(info) => Some(info.phase_one_id),
            DKGTSSPhaseFour(info) => Some(info.phase_one_id),
            ZkSaaSPhaseTwo(info) => Some(info.phase_one_id),
            _ => None,
        }
    }

    fn get_permitted_caller(self) -> Option<AccountId32> {
        use gadget_common::tangle_runtime::jobs::JobType::*;
        match self {
            DKGTSSPhaseOne(p) => p.permitted_caller,
            ZkSaaSPhaseOne(p) => p.permitted_caller,
            _ => None,
        }
    }
}

impl<
    JobId,
    MaxParticipants,
    MaxKeyLen,
    MaxDataLen,
    MaxSignatureLen,
    MaxSubmissionLen,
    MaxProofLen,
    MaxAdditionalParamsLen,
> PhaseResultExt
for tangle_runtime::PhaseResult<
    AccountId32,
    JobId,
    MaxParticipants,
    MaxKeyLen,
    MaxDataLen,
    MaxSignatureLen,
    MaxSubmissionLen,
    MaxProofLen,
    MaxAdditionalParamsLen,
>
{
    fn participants(&self) -> Option<Vec<AccountId32>> {
        match &self.job_type {
            jobs::JobType::DKGTSSPhaseOne(p) => Some(p.participants.0.clone()),
            jobs::JobType::ZkSaaSPhaseOne(p) => Some(p.participants.0.clone()),
            _ => None,
        }
    }
    fn threshold(&self) -> Option<u8> {
        match &self.job_type {
            jobs::JobType::DKGTSSPhaseOne(p) => Some(p.threshold),
            _ => None,
        }
    }
}
*/
