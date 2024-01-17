use crate::network::ZkNetworkService;
use crate::protocol::proto_gen::ZkAsyncProtocolParameters;
use ark_circom::CircomReduction;
use ark_crypto_primitives::snark::SNARK;
use ark_ec::pairing::Pairing;
use ark_ec::CurveGroup;
use ark_ff::Zero;
use ark_groth16::{Groth16, ProvingKey};
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_relations::r1cs::SynthesisError;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use async_trait::async_trait;
use futures_util::TryFutureExt;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::message::GadgetProtocolMessage;
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::{Job, TangleGadgetProtocol, WorkManagerConfig};
use gadget_common::protocol::AsyncProtocol;
use gadget_common::{BlockImportNotification, Error, FinalityNotification};
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use groth16::proving_key::PackedProvingKeyShare;
use mpc_net::{MpcNet, MultiplexedStreamID};
use pallet_jobs_rpc_runtime_api::JobsApi;
use sc_client_api::Backend;
use secret_sharing::pss::PackedSharingParams;
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::collections::HashMap;
use tangle_primitives::jobs::{
    ArkworksProofResult, HyperData, JobId, JobResult, JobType, ZkSaaSPhaseTwoRequest,
    ZkSaaSProofResult, ZkSaaSSystem,
};
use tangle_primitives::roles::RoleType;
use tangle_primitives::verifier::to_field_elements;
use tokio::sync::mpsc::UnboundedReceiver;

pub mod proto_gen;

type F = ark_bn254::Fr;
type E = ark_bn254::Bn254;

pub struct ZkProtocol<B: Block, C, BE> {
    pub client: JobsClient<B, BE, C>,
    pub account_id: AccountId,
    pub network: ZkNetworkService,
    pub logger: DebugLogger,
}

pub trait AdditionalProtocolParams: Send + Sync + Clone + 'static {
    fn n_parties(&self) -> usize;
    fn party_id(&self) -> u32;
}

#[async_trait]
impl<B, C, BE> TangleGadgetProtocol<B> for ZkProtocol<B, C, BE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
    B: Block,
    C: ClientWithApi<B, BE> + 'static,
    BE: Backend<B> + 'static,
{
    async fn get_next_jobs(
        &self,
        notification: &FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<Option<Vec<Job>>, Error> {
        self.logger
            .info(format!("Received a finality notification at {now}"));

        let jobs = self
            .client
            .query_jobs_by_validator(notification.hash, self.account_id)
            .await?;

        let mut ret = vec![];

        for job in jobs {
            let role_type = job.job_type.get_role_type();
            if matches!(job.job_type, JobType::ZkSaaSPhaseTwo(..)) {
                let phase_one_id = job
                    .job_type
                    .get_phase_one_id()
                    .expect("Should exist for a phase2 job");
                let phase_one_job = self
                    .client
                    .query_job_result(notification.hash, role_type, phase_one_id)
                    .await
                    .map_err(|err| crate::Error::ClientError {
                        err: format!("Failed to query phase one by id: {err:?}"),
                    })?
                    .ok_or_else(|| crate::Error::JobError {
                        err: JobError {
                            reason: "Phase one job not found".to_string(),
                        },
                    })?;

                let JobType::ZkSaaSPhaseOne(phase_one) = phase_one_job.job_type else {
                    return Err(Error::JobError {
                        err: JobError {
                            reason: "Phase one job type not ZkSaaS".to_string(),
                        },
                    });
                };

                let JobType::ZkSaaSPhaseTwo(phase_two) = job.job_type else {
                    return Err(Error::JobError {
                        err: JobError {
                            reason: "Phase two job type not ZkSaaS".to_string(),
                        },
                    });
                };

                let participants = phase_one.participants;

                if participants.contains(&self.account_id) {
                    let task_id = job.job_id.to_be_bytes();
                    let task_id = sp_core::keccak_256(&task_id);

                    if job_manager.job_exists(&task_id) {
                        continue;
                    }

                    let session_id = 0;
                    let retry_id = job_manager
                        .latest_retry_id(&task_id)
                        .map(|r| r + 1)
                        .unwrap_or(0);

                    let job_specific_params = ZkJobAdditionalParams {
                        n_parties: participants.len(),
                        party_id: participants
                            .iter()
                            .position(|p| p == &self.account_id)
                            .expect("Should exist") as _,
                        job_id: job.job_id,
                        role_type,
                        system: phase_one.system,
                        request: phase_two.request,
                        participants,
                    };

                    let job = self
                        .create(session_id, now, retry_id, task_id, job_specific_params)
                        .await?;

                    ret.push(job);
                }
            }
        }

        Ok(Some(ret))
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn process_error(&self, error: Error, _job_manager: &ProtocolWorkManager<WorkManager>) {
        log::error!(target: "gadget", "Received an error: {error:?}");
    }

    fn logger(&self) -> &DebugLogger {
        &self.logger
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: None,
            max_active_tasks: 2,
            max_pending_tasks: 10,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ZkJobAdditionalParams {
    n_parties: usize,
    party_id: u32,
    job_id: JobId,
    role_type: RoleType,
    system: ZkSaaSSystem,
    request: ZkSaaSPhaseTwoRequest,
    participants: Vec<AccountId>,
}

impl AdditionalProtocolParams for ZkJobAdditionalParams {
    fn n_parties(&self) -> usize {
        self.n_parties
    }
    fn party_id(&self) -> u32 {
        self.party_id
    }
}

#[async_trait]
impl<B: Block, C: ClientWithApi<B, BE> + 'static, BE: Backend<B> + 'static> AsyncProtocol
    for ZkProtocol<B, C, BE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = ZkJobAdditionalParams;

    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let client = self.client.clone();
        let rxs = zk_setup_rxs(additional_params.n_parties(), protocol_message_rx).await?;
        let other_network_ids = zk_setup_phase_order_participants(
            additional_params.participants.clone(),
            &self.network,
        )
        .map_err(|err| JobError {
            reason: format!("Failed to setup phase order participants: {err:?}"),
        })?;

        let params = ZkAsyncProtocolParameters::<_, _, _, B, BE> {
            associated_block_id,
            associated_retry_id,
            associated_session_id,
            associated_task_id,
            rxs,
            party_id: additional_params.party_id(),
            n_parties: additional_params.n_parties(),
            my_network_id: self.network.my_id(),
            other_network_ids,
            network: self.network.clone(),
            client: self.client.clone(),
            extra_parameters: additional_params.clone(),
            _pd: Default::default(),
        };

        Ok(JobBuilder::new()
            .protocol(async move {
                log::debug!(
                    "Running ZkSaaS for {:?} with JobId: {}",
                    params.extra_parameters.role_type,
                    params.extra_parameters.job_id
                );
                let ZkSaaSSystem::Groth16(ref system) = params.extra_parameters.system;
                let ZkSaaSPhaseTwoRequest::Groth16(ref job) = params.extra_parameters.request;
                let HyperData::Raw(ref proving_key_bytes) = system.proving_key else {
                    return Err(JobError {
                        reason: "Only raw proving key is supported".to_string(),
                    });
                };

                let pk = ProvingKey::<E>::deserialize_compressed(&proving_key_bytes[..]).map_err(
                    |err| JobError {
                        reason: format!("Failed to deserialize proving key: {err:?}"),
                    },
                )?;
                let l = params.n_parties / 4;
                let pp = PackedSharingParams::new(l);
                let crs_shares =
                    PackedProvingKeyShare::<E>::pack_from_arkworks_proving_key(&pk, pp);
                let our_qap_share =
                    job.qap_shares
                        .get(params.party_id as usize)
                        .ok_or_else(|| JobError {
                            reason: "Failed to get our qap share".to_string(),
                        })?;
                let HyperData::Raw(ref qap_a) = our_qap_share.a else {
                    return Err(JobError {
                        reason: "Only raw qap_a is supported".to_string(),
                    });
                };
                let HyperData::Raw(ref qap_b) = our_qap_share.b else {
                    return Err(JobError {
                        reason: "Only raw qap_b is supported".to_string(),
                    });
                };
                let HyperData::Raw(ref qap_c) = our_qap_share.c else {
                    return Err(JobError {
                        reason: "Only raw qap_c is supported".to_string(),
                    });
                };
                let our_a_share =
                    job.a_shares
                        .get(params.party_id as usize)
                        .ok_or_else(|| JobError {
                            reason: "Failed to get our a share".to_string(),
                        })?;
                let HyperData::Raw(a_share_bytes) = our_a_share else {
                    return Err(JobError {
                        reason: "Only raw a_share is supported".to_string(),
                    });
                };

                let our_ax_share =
                    job.ax_shares
                        .get(params.party_id as usize)
                        .ok_or_else(|| JobError {
                            reason: "Failed to get our ax share".to_string(),
                        })?;

                let HyperData::Raw(ax_share_bytes) = our_ax_share else {
                    return Err(JobError {
                        reason: "Only raw ax_share is supported".to_string(),
                    });
                };
                let m = system.num_inputs + system.num_constraints;
                let domain = Radix2EvaluationDomain::<F>::new(m as usize)
                    .ok_or(SynthesisError::PolynomialDegreeTooLarge)
                    .map_err(|err| JobError {
                        reason: format!("Failed to create evaluation domain: {err:?}"),
                    })?;
                let qap_share = groth16::qap::PackedQAPShare {
                    num_inputs: system.num_inputs as _,
                    num_constraints: system.num_constraints as _,
                    a: to_field_elements(qap_a).map_err(|err| JobError {
                        reason: format!("Failed to convert a to field elements: {err:?}"),
                    })?,
                    b: to_field_elements(qap_b).map_err(|err| JobError {
                        reason: format!("Failed to convert b to field elements: {err:?}"),
                    })?,
                    c: to_field_elements(qap_c).map_err(|err| JobError {
                        reason: format!("Failed to convert c to field elements: {err:?}"),
                    })?,
                    domain,
                };
                let a_share = to_field_elements(a_share_bytes).map_err(|err| JobError {
                    reason: format!("Failed to convert a_share to field elements: {err:?}"),
                })?;
                let ax_share = to_field_elements(ax_share_bytes).map_err(|err| JobError {
                    reason: format!("Failed to convert ax_share to field elements: {err:?}"),
                })?;
                let crs_share =
                    crs_shares
                        .get(params.party_id as usize)
                        .ok_or_else(|| JobError {
                            reason: "Failed to get crs share".to_string(),
                        })?;
                let h_share = groth16::ext_wit::circom_h(qap_share, &pp, &params)
                    .map_err(|err| JobError {
                        reason: format!("Failed to compute circom_h: {err:?}"),
                    })
                    .await?;
                let pi_a_share = groth16::prove::A::<E> {
                    L: Default::default(),
                    N: Default::default(),
                    r: <E as Pairing>::ScalarField::zero(),
                    pp: &pp,
                    S: &crs_share.s,
                    a: &a_share,
                }
                .compute(&params, MultiplexedStreamID::Zero)
                .map_err(|err| JobError {
                    reason: format!("Failed to compute pi_a_share: {err:?}"),
                })
                .await?;
                let pi_b_share = groth16::prove::B::<E> {
                    Z: Default::default(),
                    K: Default::default(),
                    s: <E as Pairing>::ScalarField::zero(),
                    pp: &pp,
                    V: &crs_share.v,
                    a: &a_share,
                }
                .compute(&params, MultiplexedStreamID::Zero)
                .map_err(|err| JobError {
                    reason: format!("Failed to compute pi_b_share: {err:?}"),
                })
                .await?;
                let pi_c_share = groth16::prove::C::<E> {
                    W: &crs_share.w,
                    U: &crs_share.u,
                    A: pi_a_share,
                    M: Default::default(),
                    r: <E as Pairing>::ScalarField::zero(),
                    s: <E as Pairing>::ScalarField::zero(),
                    pp: &pp,
                    H: &crs_share.h,
                    a: &a_share,
                    ax: &ax_share,
                    h: &h_share,
                }
                .compute(&params)
                .map_err(|err| JobError {
                    reason: format!("Failed to compute pi_c_share: {err:?}"),
                })
                .await?;
                if params.is_king() {
                    let (mut a, mut b, c) = (pi_a_share, pi_b_share, pi_c_share);
                    // These elements are needed to construct the full proof, they are part of the proving key.
                    // however, we can just send these values to the client, not the full proving key.
                    a += pk.a_query[0] + pk.vk.alpha_g1;
                    b += pk.b_g2_query[0] + pk.vk.beta_g2;

                    let proof = ark_groth16::Proof::<E> {
                        a: a.into_affine(),
                        b: b.into_affine(),
                        c: c.into_affine(),
                    };

                    // Verify the proof
                    // convert the public inputs from string to bigints
                    let public_inputs =
                        to_field_elements(&job.public_input).map_err(|err| JobError {
                            reason: format!(
                                "Failed to convert public inputs to field elements: {err:?}"
                            ),
                        })?;
                    let pvk = ark_groth16::prepare_verifying_key(&pk.vk);
                    let verified = Groth16::<E, CircomReduction>::verify_with_processed_vk(
                        &pvk,
                        public_inputs.as_slice(),
                        &proof,
                    )
                    .unwrap();
                    if verified {
                        log::info!("Proof verified");
                    } else {
                        log::error!("Proof verification failed");
                    }
                    let mut proof_bytes = Vec::new();
                    proof.serialize_compressed(&mut proof_bytes).unwrap();
                    let result =
                        ZkSaaSProofResult::Arkworks(ArkworksProofResult { proof: proof_bytes });

                    client
                        .submit_job_result(
                            params.extra_parameters.role_type,
                            params.extra_parameters.job_id,
                            JobResult::ZkSaaSPhaseTwo(result),
                        )
                        .await
                        .map_err(|err| JobError {
                            reason: err.to_string(),
                        })?;
                }
                Ok(())
            })
            .build())
    }
}

async fn zk_setup_rxs(
    n_parties: usize,
    mut protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
) -> Result<
    HashMap<u32, Vec<tokio::sync::Mutex<UnboundedReceiver<proto_gen::MpcNetMessage>>>>,
    JobError,
> {
    let mut txs = HashMap::new();
    let mut rxs = HashMap::new();
    for peer_id in 0..n_parties {
        // Create 3 multiplexed channels
        let mut txs_for_this_peer = vec![];
        let mut rxs_for_this_peer = vec![];
        for _ in 0..3 {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            txs_for_this_peer.push(tx);
            rxs_for_this_peer.push(tokio::sync::Mutex::new(rx));
        }

        txs.insert(peer_id as u32, txs_for_this_peer);
        rxs.insert(peer_id as u32, rxs_for_this_peer);
    }

    tokio::task::spawn(async move {
        while let Some(message) = protocol_message_rx.recv().await {
            let message: GadgetProtocolMessage = message;
            match bincode2::deserialize::<proto_gen::MpcNetMessage>(&message.payload) {
                Ok(deserialized) => {
                    let (source, sid) = (deserialized.source, deserialized.sid);
                    if let Some(txs) = txs.get(&source) {
                        if let Some(tx) = txs.get(sid as usize) {
                            if let Err(err) = tx.send(deserialized) {
                                log::warn!(
                                    "Failed to forward message from {source} to stream {sid:?} because {err:?}",
                                );
                            }
                        } else {
                            log::warn!(
                                "Failed to forward message from {source} to stream {sid:?} because the tx handle was not found",
                            );
                        }
                    } else {
                        log::warn!(
                            "Failed to forward message from {source} to stream {sid:?} because the tx handle was not found",
                        );
                    }
                }
                Err(err) => {
                    log::warn!("Failed to deserialize protocol message: {err:?}");
                }
            }
        }

        log::warn!("Async protocol message_rx died")
    });
    Ok(rxs)
}

fn zk_setup_phase_order_participants(
    mut participants: Vec<AccountId>,
    network: &ZkNetworkService,
) -> Result<HashMap<u32, AccountId>, gadget_common::Error> {
    let king_id = network
        .king_id()
        .ok_or_else(|| gadget_common::Error::ClientError {
            err: "King id not found".to_string(),
        })?;

    // The king should be moved into the 0th index of the participants, swapping positions with the 0th participant
    let king_index = participants
        .iter()
        .position(|p| p == &king_id)
        .ok_or_else(|| Error::ClientError {
            err: "King not found in participants".to_string(),
        })?;

    let current_0th = participants[0];
    participants[0] = king_id;
    participants[king_index] = current_0th;

    Ok(participants
        .into_iter()
        .enumerate()
        .map(|(i, p)| (i as u32, p))
        .collect())
}
