#[cfg(test)]
mod tests {
    use mp_ecdsa_protocol::mock::{id_to_public, new_test_ext, Jobs, Runtime, RuntimeOrigin};
    use pallet_jobs::{SubmittedJobs, SubmittedJobsRole};
    use std::time::Duration;
    use tangle_primitives::jobs::{
        DKGTSSPhaseOneJobType, DKGTSSPhaseTwoJobType, JobId, JobSubmission, JobType,
    };
    use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
    use test_utils::sync::substrate_test_channel::MultiThreadedTestExternalities;
    use tracing_subscriber::fmt::SubscriberBuilder;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    pub fn setup_log() {
        let _ = SubscriberBuilder::default()
            .with_env_filter(EnvFilter::from_default_env())
            .finish()
            .try_init();
    }

    fn remove_job(role_type: RoleType, job_id: JobId) {
        SubmittedJobs::<Runtime>::remove(role_type, job_id);
        SubmittedJobsRole::<Runtime>::remove(job_id);
    }

    async fn wait_for_job_completion(
        ext: &MultiThreadedTestExternalities,
        role_type: RoleType,
        job_id: JobId,
    ) {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if ext
                .execute_with_async(move || {
                    if Jobs::known_results(role_type, job_id).is_some() {
                        remove_job(role_type, job_id);
                        true
                    } else {
                        false
                    }
                })
                .await
            {
                return;
            }
        }
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
                    job_type: JobType::DKGTSSPhaseOne(DKGTSSPhaseOneJobType {
                        participants: identities.clone(),
                        threshold: T as _,
                        permitted_caller: None,
                        role_type: ThresholdSignatureRoleType::TssGG20,
                    }),
                };

                // Should succeed
                assert!(
                    Jobs::submit_job(RuntimeOrigin::signed(identities[0].clone()), submission)
                        .is_ok()
                );
                log::info!(target: "gadget", "******* Submitted Keygen Job {job_id}");

                let jobs_res = Jobs::query_jobs_by_validator(identities[0].clone());
                log::info!(target: "gadget", "******* Queried Jobs: {jobs_res:?}");
                assert_eq!(jobs_res.expect("Should exist").len(), 1);
                log::info!(target: "gadget", "******* Finished initial test block");
                job_id
            })
            .await;

        wait_for_job_completion(
            ext,
            RoleType::Tss(ThresholdSignatureRoleType::TssGG20),
            job_id,
        )
        .await;
        job_id
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_externalities_gadget_starts() {
        setup_log();
        new_test_ext::<1>().await.execute_with(|| {
            assert_eq!(1, 1);
        })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_externalities_keygen() {
        setup_log();
        const N: usize = 3;
        const T: usize = N - 1;

        std::panic::set_hook(Box::new(|info| {
            log::error!(target: "gadget", "Panic occurred: {info:?}");
            std::process::exit(1);
        }));

        let ext = new_test_ext::<N>().await;
        assert_eq!(wait_for_keygen::<N, T>(&ext).await, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_externalities_signing() {
        setup_log();
        const N: usize = 3;
        const T: usize = N - 1;

        std::panic::set_hook(Box::new(|info| {
            log::error!(target: "gadget", "Panic occurred: {info:?}");
            std::process::exit(1);
        }));

        let ext = new_test_ext::<N>().await;
        let keygen_job_id = wait_for_keygen::<N, T>(&ext).await;

        // Trigger a signing job by submitting a phase two job
        let job_id = ext
            .execute_with_async(move || {
                let job_id = Jobs::next_job_id();
                let identities = (0..N).map(|i| id_to_public(i as u8)).collect::<Vec<_>>();
                let submission = JobSubmission {
                    expiry: 100,
                    job_type: JobType::DKGTSSPhaseTwo(DKGTSSPhaseTwoJobType {
                        phase_one_id: keygen_job_id,
                        submission: Vec::from("Hello, world!"),
                        role_type: ThresholdSignatureRoleType::TssGG20,
                    }),
                };

                assert!(
                    Jobs::submit_job(RuntimeOrigin::signed(identities[0].clone()), submission)
                        .is_ok()
                );
                log::info!(target: "gadget", "******* Submitted Signing Job {job_id}");
                job_id
            })
            .await;

        wait_for_job_completion(
            &ext,
            RoleType::Tss(ThresholdSignatureRoleType::TssGG20),
            job_id,
        )
        .await;
    }
}
