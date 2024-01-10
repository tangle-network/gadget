#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::str::FromStr;
    use tangle_primitives::jobs::{
        Groth16System, JobSubmission, JobType, ZkSaaSPhaseOneJobType,
        ZkSaaSPhaseTwoJobType, ZkSaaSSystem,
    };
    use tangle_primitives::roles::{RoleType, ZeroKnowledgeRoleType};
    use test_utils::mock::{id_to_public, Jobs, RuntimeOrigin};
    use test_utils::sync::substrate_test_channel::MultiThreadedTestExternalities;
    use zk_saas_protocol::ZkGadgetConfig;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_zk_job() {
        test_utils::setup_log();
        const N: usize = 3;
        const T: usize = N - 1;

        let ext = new_test_ext::<N>().await;
        wait_for_job(&ext).await;
    }

    async fn wait_for_job<const N: usize, const T: usize>(ext: &MultiThreadedTestExternalities) {
        let job_id = ext.execute_with_async(|| {
            let phase_one_id = Jobs::next_job_id();
            let identities = (0..N).map(|i| id_to_public(i as u8)).collect::<Vec<_>>();

            let phase1_submission = JobSubmission {
                expiry: 100,
                ttl: 100,
                job_type: JobType::ZkSaaSPhaseOne(ZkSaaSPhaseOneJobType {
                    participants: identities.clone(),
                    permitted_caller: None,
                    system: ZkSaaSSystem::Groth16(Groth16System {
                        circuit: (),
                        num_inputs: 0,
                        num_constraints: 0,
                        proving_key: (),
                        verifying_key: vec![],
                        wasm: (),
                    }),
                    role_type: ZeroKnowledgeRoleType::ZkSaaSGroth16,
                }),
            };

            assert!(Jobs::submit_job(RuntimeOrigin::signed(identities[0]), phase1_submission).is_ok());

            let phase_two_id = Jobs::next_job_id();
            let phase2_submission = JobSubmission {
                expiry: 100,
                ttl: 100,
                job_type: JobType::ZkSaaSPhaseTwo(ZkSaaSPhaseTwoJobType {
                    phase_one_id,
                    request: (),
                    role_type: ZeroKnowledgeRoleType::ZkSaaSGroth16,
                }),
            };

            assert!(Jobs::submit_job(RuntimeOrigin::signed(identities[0]), phase2_submission).is_ok());

            log::info!(target: "gadget", "******* Submitted ZkSaaS Job {phase_one_id} & {phase_two_id}");
            phase_two_id
        }).await;

        test_utils::wait_for_job_completion(
            ext,
            RoleType::ZkSaaS(ZeroKnowledgeRoleType::ZkSaaSGroth16),
            job_id,
        )
        .await;
    }

    async fn new_test_ext<const N: usize>() -> MultiThreadedTestExternalities {
        let king_addr = SocketAddr::from_str("127.0.0.1:12344").expect("Should be valid addr");
        let king_cert =
            rcgen::generate_simple_self_signed(vec!["localhost".into(), "127.0.0.1".into()])
                .unwrap();
        let king_public_identity_der = king_cert.serialize_der().expect("Should serialize");
        let king_private_identity_der = king_cert.serialize_private_key_der();

        test_utils::mock::new_test_ext::<N, 1, _, _>(|mut node_info| async move {
            let (king_bind_addr, client_only_king_addr, client_only_king_public_identity_der) =
                if node_info.node_index == 0 {
                    (Some(king_addr), None, None)
                } else {
                    (
                        None,
                        Some(king_addr),
                        Some(king_public_identity_der.clone()),
                    )
                };

            let (public_identity_der, private_identity_der) = if node_info.node_index == 0 {
                (king_public_identity_der, king_private_identity_der)
            } else {
                let cert = rcgen::generate_simple_self_signed(vec![
                    "localhost".into(),
                    "127.0.0.1".into(),
                ])
                .unwrap();
                (
                    cert.serialize_der().expect("Should serialize"),
                    cert.serialize_private_key_der(),
                )
            };

            let config = ZkGadgetConfig {
                king_bind_addr,
                client_only_king_addr,
                public_identity_der,
                private_identity_der,
                client_only_king_public_identity_der,
                account_id: node_info.account_id,
            };

            let logger = node_info.logger.clone();
            let client = node_info.mock_clients.pop().expect("Should have client");
            let pallet_tx = node_info.pallet_tx;

            if let Err(err) = zk_saas_protocol::run(config, logger.clone(), client, pallet_tx).await
            {
                log::error!(target: "gadget", "Failed to run zk protocol: {err:?}");
            }
        })
        .await
    }
}
