#[cfg(test)]
mod tests {
    #[cfg(feature = "parallel")]
    use rayon::prelude::*;

    use ark_bn254::{Bn254, Fr as Bn254Fr};
    use ark_circom::{CircomBuilder, CircomConfig, CircomReduction};
    use ark_crypto_primitives::snark::SNARK;
    use ark_ec::pairing::Pairing;
    use ark_ff::{BigInt, Zero};
    use ark_groth16::Groth16;
    use ark_poly::Radix2EvaluationDomain;
    use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
    use ark_serialize::CanonicalSerialize;
    use ark_std::rand::SeedableRng;
    use ark_std::{cfg_chunks, cfg_into_iter};
    use frame_support::assert_ok;
    use secret_sharing::pss::PackedSharingParams;
    use std::net::SocketAddr;
    use std::str::FromStr;
    use std::sync::Arc;
    use tangle_primitives::jobs::{
        Groth16ProveRequest, Groth16System, HyperData, JobResult, JobSubmission, JobType, QAPShare,
        ZkSaaSCircuitResult, ZkSaaSPhaseOneJobType, ZkSaaSPhaseTwoJobType, ZkSaaSPhaseTwoRequest,
        ZkSaaSSystem,
    };
    use tangle_primitives::roles::{RoleType, ZeroKnowledgeRoleType};
    use tangle_primitives::verifier::from_field_elements;
    use test_utils::mock::{id_to_public, Jobs, MockBackend, RuntimeOrigin};
    use test_utils::sync::substrate_test_channel::MultiThreadedTestExternalities;
    use zk_saas_protocol::ZkGadgetConfig;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_zk_job() {
        test_utils::setup_log();
        const N: usize = 4;

        let ext = new_test_ext::<N>().await;
        wait_for_job::<N>(&ext).await;
    }

    async fn wait_for_job<const N: usize>(ext: &MultiThreadedTestExternalities) {
        let job_id = ext.execute_with_async(|| {
            let phase_one_id = Jobs::next_job_id();
            let identities = (0..N).map(|i| id_to_public(i as u8)).collect::<Vec<_>>();
            let wasm_file = std::fs::read("../../fixtures/sha256/sha256_js/sha256.wasm").unwrap();
            let r1cs_file = std::fs::read("../../fixtures/sha256/sha256.r1cs").unwrap();
            let cfg = CircomConfig::<Bn254>::new(
                "../../fixtures/sha256/sha256_js/sha256.wasm",
                "../../fixtures/sha256/sha256.r1cs",
            )
            .unwrap();
            let mut builder = CircomBuilder::new(cfg);
            let rng = &mut ark_std::rand::rngs::StdRng::from_seed([42u8; 32]);
            builder.push_input("a", 1);
            builder.push_input("b", 2);
            let circuit = builder.setup();
            let (pk, vk) =
                Groth16::<Bn254, CircomReduction>::circuit_specific_setup(circuit, rng)
                    .unwrap();
            let mut pk_bytes = Vec::new();
            pk.serialize_compressed(&mut pk_bytes).unwrap();
            let mut vk_bytes = Vec::new();
            vk.serialize_compressed(&mut vk_bytes).unwrap();
            let circom = builder.build().unwrap();
            let full_assignment = circom.witness.clone().unwrap();
            let cs = ConstraintSystem::<Bn254Fr>::new_ref();
            circom.generate_constraints(cs.clone()).unwrap();
            assert!(cs.is_satisfied().unwrap());
            let matrices = cs.to_matrices().unwrap();

            let num_inputs = matrices.num_instance_variables;
            let num_constraints = matrices.num_constraints;
            let phase1_submission = JobSubmission {
                expiry: 100,
                ttl: 100,
                job_type: JobType::ZkSaaSPhaseOne(ZkSaaSPhaseOneJobType {
                    participants: identities.clone().try_into().unwrap(),
                    permitted_caller: None,
                    system: ZkSaaSSystem::Groth16(Groth16System {
                        circuit: HyperData::Raw(r1cs_file.try_into().unwrap()),
                        num_inputs: num_inputs as u64,
                        num_constraints: num_constraints as u64,
                        proving_key: HyperData::Raw(pk_bytes.try_into().unwrap()),
                        verifying_key: vk_bytes.try_into().unwrap(),
                        wasm: HyperData::Raw(wasm_file.try_into().unwrap()),
                    }),
                    role_type: ZeroKnowledgeRoleType::ZkSaaSGroth16,
                }),
            };

            assert_ok!(Jobs::submit_job(RuntimeOrigin::signed(identities[0]), phase1_submission));
            let phase1_result = JobResult::ZkSaaSPhaseOne(ZkSaaSCircuitResult { job_id: phase_one_id, participants: identities.clone().try_into().unwrap() });
            assert_ok!(
                Jobs::submit_job_result(
                RuntimeOrigin::signed(identities[0]),
                RoleType::ZkSaaS(ZeroKnowledgeRoleType::ZkSaaSGroth16),
                phase_one_id,
                phase1_result
            ));

            let pp = PackedSharingParams::new(N / 4);
            let qap = groth16::qap::qap::<Bn254Fr, Radix2EvaluationDomain<_>>(&matrices, &full_assignment).unwrap();
            let qap_shares = qap.pss(&pp)
                .into_iter()
                .map(|s| QAPShare {
                    a: HyperData::Raw(from_field_elements(&s.a).unwrap().try_into().unwrap()),
                    b: HyperData::Raw(from_field_elements(&s.b).unwrap().try_into().unwrap()),
                    c: HyperData::Raw(from_field_elements(&s.c).unwrap().try_into().unwrap()),
                })
                .collect::<Vec<_>>();

            let aux_assignment = &full_assignment[num_inputs..];
            let ax_shares = pack_from_witness::<Bn254>(&pp, aux_assignment.to_vec())
                .into_iter()
                .filter_map(|s| from_field_elements(&s).ok())
                .map(|r|HyperData::Raw(r.try_into().unwrap()))
                .collect::<Vec<_>>();
            let a_shares = pack_from_witness::<Bn254>(&pp, full_assignment[1..].to_vec())
                .into_iter()
                .filter_map(|s| from_field_elements(&s).ok())
                .map(|r| HyperData::Raw(r.try_into().unwrap()))
                .collect::<Vec<_>>();
            let public_input = from_field_elements::<Bn254Fr>(&[BigInt!("72587776472194017031617589674261467945970986113287823188107011979").into()]).unwrap().try_into().unwrap();
            let phase_two_id = Jobs::next_job_id();
            let phase2_submission = JobSubmission {
                expiry: 100,
                ttl: 100,
                job_type: JobType::ZkSaaSPhaseTwo(ZkSaaSPhaseTwoJobType {
                    phase_one_id,
                    request: ZkSaaSPhaseTwoRequest::Groth16(
                        Groth16ProveRequest {
                            public_input,
                            a_shares: a_shares.try_into().unwrap(),
                            ax_shares: ax_shares.try_into().unwrap(),
                            qap_shares: qap_shares.try_into().unwrap(),
                        }
                    ),
                    role_type: ZeroKnowledgeRoleType::ZkSaaSGroth16,
                }),
            };

            assert_ok!(Jobs::submit_job(RuntimeOrigin::signed(identities[0]), phase2_submission));

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
        let king_cert = Arc::new(king_cert);
        test_utils::mock::new_test_ext::<N, 1, _, _, _>(
            king_cert,
            |king_cert, mut node_info| async move {
                let king_public_identity_der = king_cert.serialize_der().expect("Should serialize");
                let king_private_identity_der = king_cert.serialize_private_key_der();
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
                    let cert =
                        rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
                    (
                        cert.serialize_der().expect("Should serialize"),
                        cert.serialize_private_key_der(),
                    )
                };

                let logger = node_info.logger.clone();
                let client = node_info.mock_clients.pop().expect("Should have client");
                let pallet_tx = node_info.pallet_tx;

                let config = ZkGadgetConfig::<_, _, MockBackend> {
                    king_bind_addr,
                    client_only_king_addr,
                    public_identity_der,
                    private_identity_der,
                    client_only_king_public_identity_der,
                    account_id: node_info.account_id,
                    logger,
                    client,
                    pallet_tx: Arc::new(pallet_tx),
                    _pd: Default::default(),
                };

                if let Err(err) = config.execute().await {
                    log::error!(target: "gadget", "Failed to run zk protocol: {err:?}");
                }
            },
        )
        .await
    }

    fn pack_from_witness<E: Pairing>(
        pp: &PackedSharingParams<E::ScalarField>,
        full_assignment: Vec<E::ScalarField>,
    ) -> Vec<Vec<E::ScalarField>> {
        let packed_assignments = cfg_chunks!(full_assignment, pp.l)
            .map(|chunk| {
                let secrets = if chunk.len() < pp.l {
                    let mut secrets = chunk.to_vec();
                    secrets.resize(pp.l, E::ScalarField::zero());
                    secrets
                } else {
                    chunk.to_vec()
                };

                let rng = &mut ark_std::test_rng();
                pp.pack(secrets, rng)
            })
            .collect::<Vec<_>>();

        cfg_into_iter!(0..pp.n)
            .map(|i| {
                cfg_into_iter!(0..packed_assignments.len())
                    .map(|j| packed_assignments[j][i])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }
}
