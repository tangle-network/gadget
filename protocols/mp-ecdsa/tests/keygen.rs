#[cfg(test)]
mod tests {
    use mp_ecdsa_protocol::mock::{id_to_public, new_test_ext, Jobs, Runtime, RuntimeOrigin};
    use pallet_jobs::SubmittedJobs;
    use std::time::Duration;
    use tangle_primitives::jobs::{
        DKGTSSPhaseOneJobType, DkgKeyType, JobKey, JobSubmission, JobType,
    };
    use tracing_subscriber::fmt::SubscriberBuilder;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    pub fn setup_log() {
        let _ = SubscriberBuilder::default()
            .with_env_filter(EnvFilter::from_default_env())
            .finish()
            .try_init();
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

        new_test_ext::<N>()
            .await
            .execute_with_async(|| {
                assert_eq!(1, 1);

                let identities = (0..N).map(|i| id_to_public(i as u8)).collect::<Vec<_>>();

                let submission = JobSubmission {
                    expiry: 100,
                    job_type: JobType::DKGTSSPhaseOne(DKGTSSPhaseOneJobType {
                        participants: identities.clone(),
                        threshold: T as u8,
                        permitted_caller: None,
                        key_type: DkgKeyType::Ecdsa,
                    }),
                };

                // Should succeed
                assert!(
                    Jobs::submit_job(RuntimeOrigin::signed(identities[0].clone()), submission)
                        .is_ok()
                );
                log::info!(target: "gadget", "******* Submitted Job");

                let jobs_res = SubmittedJobs::<Runtime>::get(JobKey::DKG, 0).expect("Should exist");

                assert_eq!(
                    jobs_res
                        .job_type
                        .get_participants()
                        .expect("Should exist")
                        .len(),
                    N
                );
                /*let jobs_res = Jobs::query_jobs_by_validator(identities[0].clone());
                log::info!(target: "gadget", "******* Queried Jobs: {jobs_res:?}");
                assert_eq!(jobs_res.expect("Should exist").len(), 1);*/
                //advance_to_block(4);
                log::info!(target: "gadget", "******* Finished initial test block");
            })
            .await;

        // TODO: loop and sleep until we get the public key on-chain
        loop {
            tokio::time::sleep(Duration::from_millis(10)).await;
            tokio::task::yield_now().await;
        }
    }
}
