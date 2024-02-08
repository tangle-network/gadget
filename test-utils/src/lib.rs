#![feature(return_position_impl_trait_in_trait)]

use crate::mock::{Jobs, NodeInput, Runtime};
use crate::sync::substrate_test_channel::MultiThreadedTestExternalities;
use gadget_common::client::{GadgetPhaseResult, JobsApiForGadget};
use gadget_common::full_protocol::FullProtocolConfig;
pub use gadget_core::job_manager::SendFuture;
use pallet_jobs::{SubmittedJobs, SubmittedJobsRole};
use sp_api::ProvideRuntimeApi;
use std::time::Duration;
use tangle_primitives::jobs::JobId;
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
) -> GadgetPhaseResult<mock::BlockNumber> {
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

pub trait ProtocolTestingUtils: FullProtocolConfig
where
    <<Self as FullProtocolConfig>::Client as ProvideRuntimeApi<
        <Self as FullProtocolConfig>::Block,
    >>::Api: JobsApiForGadget<<Self as FullProtocolConfig>::Block>,
{
    fn setup_test_node(
        additional_param: <Self as FullProtocolConfig>::AdditionalTestParameters,
        node_input: NodeInput,
    ) -> impl SendFuture<'static, ()>;
}

#[macro_export]
macro_rules! generate_tss_tests {
    ($config:ident, $threshold_sig_ty:expr) => {
        #[cfg(test)]
        mod tests {
            use std::sync::Arc;
            use tangle_primitives::jobs::{
                DKGTSSPhaseOneJobType, DKGTSSPhaseTwoJobType, JobId, JobSubmission, JobType,
            };
            use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
            use test_utils::mock::{id_to_public, Jobs, MockBackend, RuntimeOrigin};
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
                const N: usize = 3;
                const T: usize = N - 1;

                let ext = new_test_ext::<N>().await;
                assert_eq!(wait_for_keygen::<N, T>(&ext).await, 0);
            }

            #[tokio::test(flavor = "multi_thread")]
            async fn test_externalities_signing() {
                test_utils::setup_log();
                const N: usize = 3;
                const T: usize = N - 1;

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
                        let identities = (0..N).map(|i| id_to_public(i as u8)).collect::<Vec<_>>();

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

                        assert!(Jobs::submit_job(RuntimeOrigin::signed(identities[0]), submission).is_ok());

                        log::info!(target: "gadget", "******* Submitted Keygen Job {job_id}");
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
                        let identities = (0..N).map(|i| id_to_public(i as u8)).collect::<Vec<_>>();
                        let submission = JobSubmission {
                            expiry: 100,
                            ttl: 100,
                            job_type: JobType::DKGTSSPhaseTwo(DKGTSSPhaseTwoJobType {
                                phase_one_id: keygen_job_id,
                                submission: Vec::from("Hello, world!").try_into().unwrap(),
                                role_type: $threshold_sig_ty,
                            }),
                        };

                        assert!(Jobs::submit_job(RuntimeOrigin::signed(identities[0]), submission).is_ok());

                        log::info!(target: "gadget", "******* Submitted Signing Job {job_id}");
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
                test_utils::mock::new_test_ext::<N, 2, _, _, _>((), |extra_param, mut node_input| async move {
                    crate::$config::setup_test_node(extra_param, node_input).await
                })
                .await
            }
        }
    };
}
