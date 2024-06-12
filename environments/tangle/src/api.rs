use auto_impl::auto_impl;
use gadget_common::async_trait::async_trait;
use gadget_common::environments::GadgetEnvironment;
use gadget_common::tangle_runtime;
use gadget_common::tangle_runtime::{
    AccountId32, MaxAdditionalParamsLen, MaxDataLen, MaxKeyLen, MaxParticipants, MaxProofLen,
    MaxSignatureLen, MaxSubmissionLen,
};
use gadget_core::gadget::general::Client;

#[async_trait]
#[auto_impl(Arc)]
pub trait ClientWithApi<Env: GadgetEnvironment>: Client<Env::Event> + 'static {
    /// Query jobs associated with a specific validator.
    ///
    /// This function takes a `validator` parameter of type `AccountId` and attempts
    /// to retrieve a list of jobs associated with the provided validator. If successful,
    /// it constructs a vector of `RpcResponseJobsData` containing information
    /// about the jobs and returns it as a `Result`.
    ///
    /// # Arguments
    ///
    /// * `validator` - The account ID of the validator whose jobs are to be queried.
    ///
    /// # Returns
    ///
    /// An optional vec of `RpcResponseJobsData` of jobs assigned to validator
    async fn query_jobs_by_validator(
        &self,
        at: [u8; 32],
        validator: AccountId32,
    ) -> Result<
        Option<
            Vec<
                tangle_runtime::RpcResponseJobsData<
                    AccountId32,
                    u64,
                    MaxParticipants,
                    MaxSubmissionLen,
                    MaxAdditionalParamsLen,
                >,
            >,
        >,
        gadget_common::Error,
    >;
    /// Queries a job by its key and ID.
    ///
    /// # Arguments
    ///
    /// * `role_type` - The role of the job.
    /// * `job_id` - The ID of the job.
    ///
    /// # Returns
    ///
    /// An optional `RpcResponseJobsData` containing the account ID of the job.
    async fn query_job_by_id(
        &self,
        at: [u8; 32],
        role_type: tangle_runtime::RoleType,
        job_id: u64,
    ) -> Result<
        Option<
            tangle_runtime::RpcResponseJobsData<
                AccountId32,
                u64,
                MaxParticipants,
                MaxSubmissionLen,
                MaxAdditionalParamsLen,
            >,
        >,
        gadget_common::Error,
    >;

    /// Queries the result of a job by its role_type and ID.
    ///
    /// # Arguments
    ///
    /// * `role_type` - The role of the job.
    /// * `job_id` - The ID of the job.
    ///
    /// # Returns
    ///
    /// An `Option` containing the phase one result of the job, wrapped in an `PhaseResult`.
    async fn query_job_result(
        &self,
        at: [u8; 32],
        role_type: tangle_runtime::RoleType,
        job_id: u64,
    ) -> Result<
        Option<
            tangle_runtime::PhaseResult<
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
    >;

    /// Queries next job ID.
    ///
    ///  # Returns
    ///  Next job ID.
    async fn query_next_job_id(&self, at: [u8; 32]) -> Result<u64, gadget_common::Error>;

    /// Queries restaker's role key
    ///
    ///  # Returns
    ///  Role key
    async fn query_restaker_role_key(
        &self,
        at: [u8; 32],
        address: AccountId32,
    ) -> Result<Option<Vec<u8>>, gadget_common::Error>;

    /// Queries restaker's roles
    ///
    /// # Returns
    /// List of roles enabled for restaker
    async fn query_restaker_roles(
        &self,
        at: [u8; 32],
        address: AccountId32,
    ) -> Result<Vec<tangle_runtime::RoleType>, gadget_common::Error>;
}
