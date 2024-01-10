#[cfg(test)]
mod tests {
    use futures::stream::FuturesUnordered;
    use futures::StreamExt;
    use mp_ecdsa_protocol::MpEcdsaProtocolConfig;
    use pallet_jobs::{SubmittedJobs, SubmittedJobsRole};
    use std::time::Duration;
    use tangle_primitives::jobs::{
        DKGTSSPhaseOneJobType, DKGTSSPhaseTwoJobType, JobId, JobSubmission, JobType,
    };
    use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
    use test_utils::mock::{id_to_public, Jobs, MockBackend, Runtime, RuntimeOrigin};
    use test_utils::sync::substrate_test_channel::MultiThreadedTestExternalities;
    use tracing_subscriber::fmt::SubscriberBuilder;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_externalities_gadget_starts() {
        setup_log();
        new_test_ext::<1>()
            .await
            .execute_with_async(|| {
                assert_eq!(1, 1);
            })
            .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_externalities_keygen() {
        setup_log();
        const N: usize = 3;
        const T: usize = N - 1;

        let ext = new_test_ext::<N>().await;
        assert_eq!(wait_for_keygen::<N, T>(&ext).await, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_externalities_signing() {
        setup_log();
        const N: usize = 3;
        const T: usize = N - 1;

        let ext = new_test_ext::<N>().await;
        let keygen_job_id = wait_for_keygen::<N, T>(&ext).await;
        assert_eq!(wait_for_signing::<N>(&ext, keygen_job_id).await, 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_externalities_parallel_jobs() {
        setup_log();
        const N: usize = 3;
        const T: usize = N - 1;
        const TEST_COUNT: usize = 2;

        let ext = new_test_ext::<N>().await;
        let futures = FuturesUnordered::new();

        for _ in 0..TEST_COUNT {
            let ext = ext.clone();
            futures.push(Box::pin(async move {
                let keygen_job_id = wait_for_keygen::<N, T>(&ext).await;
                wait_for_signing::<N>(&ext, keygen_job_id).await;
            }));
        }

        futures.collect::<()>().await;
    }

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

    #[allow(dead_code)]
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
                .execute_with_async(move || Jobs::known_results(role_type, job_id).is_some())
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
                    ttl: 100,
                    job_type: JobType::DKGTSSPhaseOne(DKGTSSPhaseOneJobType {
                        participants: identities.clone(),
                        threshold: T as _,
                        permitted_caller: None,
                        role_type: ThresholdSignatureRoleType::TssGG20,
                    }),
                };

                assert!(Jobs::submit_job(RuntimeOrigin::signed(identities[0]), submission).is_ok());

                log::info!(target: "gadget", "******* Submitted Keygen Job {job_id}");
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
                        submission: Vec::from("Hello, world!"),
                        role_type: ThresholdSignatureRoleType::TssGG20,
                    }),
                };

                assert!(Jobs::submit_job(RuntimeOrigin::signed(identities[0]), submission).is_ok());

                log::info!(target: "gadget", "******* Submitted Signing Job {job_id}");
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

    async fn new_test_ext<const N: usize>() -> MultiThreadedTestExternalities {
        test_utils::mock::new_test_ext::<N, 2, _, _>(|mut node_input| async move {
            let keygen_client = node_input.mock_clients.pop().expect("No keygen client");
            let signing_client = node_input.mock_clients.pop().expect("No signing client");

            let keygen_network = node_input.mock_networks.pop().expect("No keygen network");
            let signing_network = node_input.mock_networks.pop().expect("No signing network");

            let config = MpEcdsaProtocolConfig {
                account_id: node_input.account_id,
            };

            let logger = node_input.logger.clone();
            let (pallet_tx, keystore) = (node_input.pallet_tx, node_input.keystore);
            logger.info("Starting gadget");
            if let Err(err) = mp_ecdsa_protocol::run::<_, MockBackend, _, _, _, _, _>(
                config,
                keygen_client,
                signing_client,
                logger.clone(),
                keystore,
                keygen_network,
                signing_network,
                pallet_tx,
            )
            .await
            {
                log::error!(target: "gadget", "Error running gadget: {err:?}");
            }
        })
        .await
    }
}
