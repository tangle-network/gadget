#[cfg(test)]
mod tests {
    use futures::stream::FuturesUnordered;
    use futures::StreamExt;
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

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "takes a long time to work on CI"]
    async fn test_externalities_parallel_jobs() {
        test_utils::setup_log();
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
                        role_type: ThresholdSignatureRoleType::ZcashFrostRistretto255,
                    }),
                };

                assert!(Jobs::submit_job(RuntimeOrigin::signed(identities[0]), submission).is_ok());

                log::info!(target: "gadget", "******* Submitted Keygen Job {job_id}");
                job_id
            })
            .await;

        test_utils::wait_for_job_completion(
            ext,
            RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostRistretto255),
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
                let submission = Vec::from("Hello, world!");
                let job_id = Jobs::next_job_id();
                let identities = (0..N).map(|i| id_to_public(i as u8)).collect::<Vec<_>>();
                let submission = JobSubmission {
                    expiry: 100,
                    ttl: 100,
                    job_type: JobType::DKGTSSPhaseTwo(DKGTSSPhaseTwoJobType {
                        phase_one_id: keygen_job_id,
                        submission,
                        role_type: ThresholdSignatureRoleType::ZcashFrostRistretto255,
                    }),
                };

                assert!(Jobs::submit_job(RuntimeOrigin::signed(identities[0]), submission).is_ok());

                log::info!(target: "gadget", "******* Submitted Signing Job {job_id}");
                job_id
            })
            .await;

        test_utils::wait_for_job_completion(
            ext,
            RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostRistretto255),
            job_id,
        )
        .await;
        job_id
    }

    async fn new_test_ext<const N: usize>() -> MultiThreadedTestExternalities {
        test_utils::mock::new_test_ext::<N, 4, _, _, _>((), |_, mut node_input| async move {
            let keygen_client = node_input.mock_clients.pop().expect("No keygen client");
            let signing_client = node_input.mock_clients.pop().expect("No signing client");

            let keygen_network = node_input.mock_networks.pop().expect("No keygen network");
            let signing_network = node_input.mock_networks.pop().expect("No signing network");
            let account_id = node_input.account_id;

            let logger = node_input.logger.clone();
            let (pallet_tx, keystore) = (node_input.pallet_tx, node_input.keystore);
            logger.info("Starting gadget");
            if let Err(err) = zcash_frost_protocol::run::<_, MockBackend, _, _, _, _>(
                account_id,
                logger.clone(),
                keystore,
                pallet_tx,
                (keygen_client, signing_client),
                (keygen_network, signing_network),
            )
            .await
            {
                log::error!(target: "gadget", "Error running gadget: {err:?}");
            }
        })
        .await
    }
}
