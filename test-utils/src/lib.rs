use crate::mock::{Jobs, Runtime};
use crate::sync::substrate_test_channel::MultiThreadedTestExternalities;
pub use gadget_core::job_manager::SendFuture;
pub use log;
use pallet_jobs::{SubmittedJobs, SubmittedJobsRole};
use std::time::Duration;
use tangle_primitives::jobs::{JobId, PhaseResult};
use tangle_primitives::roles::RoleType;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::util::SubscriberInitExt;

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
) -> PhaseResult<mock::BlockNumber> {
    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Some(ret) = ext
            .execute_with_async(move || Jobs::known_results(role_type, job_id))
            .await
        {
            return ret;
        }
    }
}

pub fn remove_job(role_type: RoleType, job_id: JobId) {
    SubmittedJobs::<Runtime>::remove(role_type, job_id);
    SubmittedJobsRole::<Runtime>::remove(job_id);
}

#[macro_export]
/// When using this macro, the `generate_setup_and_run_command` macro should be used to generate the `setup_node` function.
/// If not, you must manually create a function named `setup_node` in your crate's lib.rs that accepts a NodeInput and
/// returns a future that runs the protocols.
#[allow(clippy::crate_in_macro_def)]
macro_rules! generate_signing_and_keygen_tss_tests {
    ($t:expr, $n:expr, $proto_count:expr, $threshold_sig_ty:expr) => {
        #[cfg(test)]
        mod tests {
            use std::sync::Arc;
            use tangle_primitives::jobs::{
                DKGTSSPhaseOneJobType, DKGTSSPhaseTwoJobType, JobId, JobSubmission, JobType,
            };
            use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
            use tangle_primitives::AccountId;
            use gadget_common::full_protocol::FullProtocolConfig;
            use test_utils::mock::{id_to_public, id_to_sr25519_public, Jobs, MockBackend, RuntimeOrigin};
            use test_utils::sync::substrate_test_channel::MultiThreadedTestExternalities;

            #[tokio::test(flavor = "multi_thread")]
            async fn test_externalities_gadget_starts() {
                test_utils::setup_log();
                new_test_ext::<1>()
                    .await
                    .execute_with_async(|| {
                        assert_eq!(1, 1);
                    })
                    .await
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_externalities_keygen() {
                test_utils::setup_log();
                const N: usize = $n;
                const T: usize = $t;

                let ext = new_test_ext::<N>().await;
                assert_eq!(wait_for_keygen::<N, T>(&ext).await, 0);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_externalities_signing() {
                test_utils::setup_log();
                const N: usize = $n;
                const T: usize = $t;

                let ext = new_test_ext::<N>().await;
                let keygen_job_id = wait_for_keygen::<N, T>(&ext).await;
                assert_eq!(wait_for_signing::<N>(&ext, keygen_job_id).await, 1);
            }

            async fn wait_for_keygen<const N: usize, const T: usize>(
                ext: &MultiThreadedTestExternalities,
            ) -> JobId {
                let job_id = ext
                    .execute_with_async(|| {
                        let job_id = Jobs::next_job_id();
                        let identities = (0..N).map(|i| id_to_sr25519_public(i as u8)).map(AccountId::from).collect::<Vec<_>>();

                        let submission = JobSubmission {
                            expiry: 100,
                            ttl: 100,
                            job_type: JobType::DKGTSSPhaseOne(DKGTSSPhaseOneJobType {
                                participants: identities.clone().try_into().unwrap(),
                                threshold: T as _,
                                permitted_caller: None,
                                role_type: $threshold_sig_ty,
                            }),
                        };

                        assert!(Jobs::submit_job(RuntimeOrigin::signed(identities[0].clone()), submission).is_ok());

                        $crate::log::info!(target: "gadget", "******* Submitted Keygen Job {job_id}");
                        job_id
                    })
                    .await;

                test_utils::wait_for_job_completion(
                    ext,
                    RoleType::Tss($threshold_sig_ty),
                    job_id,
                )
                .await;
                job_id
            }

            async fn wait_for_signing<const N: usize>(
                ext: &MultiThreadedTestExternalities,
                keygen_job_id: JobId,
            ) -> JobId {
                let job_id = ext
                    .execute_with_async(move || {
                        let job_id = Jobs::next_job_id();
                        let identities = (0..N).map(|i| id_to_sr25519_public(i as u8)).map(AccountId::from).collect::<Vec<_>>();
                        let submission = JobSubmission {
                            expiry: 100,
                            ttl: 100,
                            job_type: JobType::DKGTSSPhaseTwo(DKGTSSPhaseTwoJobType {
                                phase_one_id: keygen_job_id,
                                submission: Vec::from("Hello, world!").try_into().unwrap(),
                                role_type: $threshold_sig_ty,
                            }),
                        };

                        assert!(Jobs::submit_job(RuntimeOrigin::signed(identities[0].clone()), submission).is_ok());

                        $crate::log::info!(target: "gadget", "******* Submitted Signing Job {job_id}");
                        job_id
                    })
                    .await;

                test_utils::wait_for_job_completion(
                    ext,
                    RoleType::Tss($threshold_sig_ty),
                    job_id,
                )
                .await;
                job_id
            }

            async fn new_test_ext<const N: usize>() -> MultiThreadedTestExternalities {
                test_utils::mock::new_test_ext::<N, $proto_count, (), _, _>((), crate::setup_node).await
            }
        }
    };
}
