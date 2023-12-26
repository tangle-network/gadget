#[cfg(test)]
mod tests {
    use mp_ecdsa_protocol::mock::{id_to_public, new_test_ext, Jobs, RuntimeOrigin};
    use std::time::Duration;
    use tangle_primitives::jobs::{DKGTSSPhaseOneJobType, DkgKeyType, JobSubmission, JobType};
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
        new_test_ext::<1>().execute_with(|| {
            assert_eq!(1, 1);
        })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_externalities_keygen() {
        setup_log();
        const N: usize = 3;
        const T: usize = N - 1;

        new_test_ext::<N>()
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
                //advance_to_block(4);
                log::info!(target: "gadget", "******* Submitted Job");
            })
            .await;

        tokio::time::sleep(Duration::from_millis(20000)).await;
    }
}
