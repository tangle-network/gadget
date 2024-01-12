use crate::mock::{Jobs, Runtime};
use crate::sync::substrate_test_channel::MultiThreadedTestExternalities;
use pallet_jobs::{SubmittedJobs, SubmittedJobsRole};
use std::time::Duration;
use tangle_primitives::jobs::{JobId, PhaseResult};
use tangle_primitives::roles::RoleType;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::util::SubscriberInitExt;
use gadget_common::client::AccountId;

pub mod mock;
pub mod sync;

/// Sets up the default logging as well as setting a panic hook for tests
pub fn setup_log() {
    let _ = SubscriberBuilder::default()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init();

    std::panic::set_hook(Box::new(|info| {
        log::error!(target: "gadget", "Panic occurred: {info:?}");
        std::process::exit(1);
    }));
}

pub async fn wait_for_job_completion(
    ext: &MultiThreadedTestExternalities,
    role_type: RoleType,
    job_id: JobId,
) -> PhaseResult<AccountId, u64> {
    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Some(ret) = ext
            .execute_with_async(move || Jobs::known_results(role_type, job_id))
            .await
        {
            return ret
        }
    }
}

pub fn remove_job(role_type: RoleType, job_id: JobId) {
    SubmittedJobs::<Runtime>::remove(role_type, job_id);
    SubmittedJobsRole::<Runtime>::remove(job_id);
}
