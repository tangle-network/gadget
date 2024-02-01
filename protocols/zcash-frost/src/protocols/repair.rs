use async_trait::async_trait;
use frost_core::keys::{KeyPackage, PublicKeyPackage};
use frost_ed25519::Ed25519Sha512;
use frost_p256::P256Sha256;
use frost_ristretto255::Ristretto255Sha512;
use frost_secp256k1::Secp256K1Sha256;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::{GadgetProtocol, JobInitMetadata, WorkManagerConfig};
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use gadget_common::protocol::AsyncProtocol;
use gadget_common::{Block, BlockImportNotification};
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use pallet_jobs_rpc_runtime_api::JobsApi;
use rand::SeedableRng;
use round_based::MpcParty;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_core::keccak_256;
use std::collections::HashMap;
use std::sync::Arc;
use tangle_primitives::jobs::{
    DKGTSSSignatureResult, DigitalSignatureType, JobId, JobResult, JobType,
};
use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::rounds;
use crate::rounds::keygen::FrostKeyShare;

pub struct ZcashFrostRepairProtocol<B: Block, BE, KBE: KeystoreBackend, C, N> {
    client: JobsClient<B, BE, C>,
    key_store: ECDSAKeyStore<KBE>,
    network: N,
    logger: DebugLogger,
    account_id: AccountId,
}

pub async fn create_protocol<B, BE, KBE, C, N>(
    account_id: AccountId,
    client: JobsClient<B, BE, C>,
    network: N,
    logger: DebugLogger,
    key_store: ECDSAKeyStore<KBE>,
) -> ZcashFrostRepairProtocol<B, BE, KBE, C, N>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    ZcashFrostRepairProtocol {
        client,
        network,
        key_store,
        logger,
        account_id,
    }
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        KBE: KeystoreBackend,
        N: Network,
    > GadgetProtocol<B, BE, C> for ZcashFrostRepairProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    fn name(&self) -> String {
        "zcash-frost-repair".to_string()
    }

    async fn create_next_job(
        &self,
        job: JobInitMetadata<B>,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, gadget_common::Error> {
        let job_id = job.job_id;

        let JobType::DKGTSSPhaseThree(p3_job) = job.job_type else {
            panic!("Should be valid type")
        };
        let previous_job_id = p3_job.phase_one_id;

        let phase1_job = job.phase1_job.expect("Should exist for a phase 2 job");
        let participants = phase1_job.clone().get_participants().expect("Should exist");
        let t = phase1_job.get_threshold().expect("Should exist") as u16;

        let seed =
            keccak_256(&[&job_id.to_be_bytes()[..], &job.retry_id.to_be_bytes()[..]].concat());
        let mut rng = rand_chacha::ChaChaRng::from_seed(seed);

        let key = self
            .key_store
            .get_job_result(previous_job_id)
            .await
            .map_err(|err| gadget_common::Error::ClientError {
                err: err.to_string(),
            })?
            .ok_or_else(|| gadget_common::Error::ClientError {
                err: format!("No key found for job ID: {job_id:?}"),
            })?;

        let user_id_to_account_id_mapping = Arc::new(
            participants
                .clone()
                .into_iter()
                .enumerate()
                .map(|r| (r.0 as UserID, r.1))
                .collect(),
        );

        let params = ZcashFrostRepairExtraParams {
            i: participants
                .iter()
                .position(|p| p == &self.account_id)
                .expect("Should exist") as u16,
            t,
            helpers: participants
                .into_iter()
                .enumerate()
                .map(|r| r.0 as u16)
                .collect(),
            job_id,
            // TODO: Update to use the correct participant once the job type is updated.
            participant: 1u16,
            role_type: job.role_type,
            keyshare: key,
            user_id_to_account_id_mapping,
        };
        Ok(params)
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<(), gadget_common::Error> {
        Ok(())
    }

    async fn process_error(
        &self,
        error: gadget_common::Error,
        _job_manager: &ProtocolWorkManager<WorkManager>,
    ) {
        log::error!(target: "gadget", "Error: {error:?}");
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn role_filter(&self, role: RoleType) -> bool {
        matches!(
            role,
            RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostEd25519)
                | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostP256)
                | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostRistretto255)
                | RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostSecp256k1)
        )
    }

    fn phase_filter(&self, job: JobType<AccountId>) -> bool {
        matches!(job, JobType::DKGTSSPhaseTwo(_))
    }

    fn client(&self) -> &JobsClient<B, BE, C> {
        &self.client
    }

    fn logger(&self) -> &DebugLogger {
        &self.logger
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: Some(crate::constants::signing_worker::JOB_POLL_INTERVAL),
            max_active_tasks: crate::constants::signing_worker::MAX_RUNNING_TASKS,
            max_pending_tasks: crate::constants::signing_worker::MAX_ENQUEUED_TASKS,
        }
    }
}

macro_rules! run_threshold_repair {
    ($impl_type:ty, $keyshare:expr, $tracer:expr, $i:expr, $helpers:expr, $participant:expr, $role:expr, $rng:expr, $party:expr) => {{
        let key_package =
            KeyPackage::<$impl_type>::deserialize(&$keyshare.key_package).map_err(|err| {
                JobError {
                    reason: format!("Failed to deserialize key share: {err:?}"),
                }
            })?;

        let public_key_package = PublicKeyPackage::<$impl_type>::deserialize(
            &$keyshare.pubkey_package,
        )
        .map_err(|err| JobError {
            reason: format!("Failed to deserialize public key package: {err:?}"),
        })?;

        let secret_share = SecretShare::<$impl_type> {
            header: Header::default(),
            identifier: round2_secret_package.identifier,
            signing_share: key_package.signing_share,
            commitment: commitment.clone(),
        };

        rounds::repair::run_threshold_repair::<$impl_type, _, _>(
            Some($tracer),
            $i,
            $helpers,
            $key_package.secret_share,
            $commitment,
            $participant,
            $role,
            $rng,
            $party,
        )
        .await
        .map_err(|err| JobError {
            reason: format!("Failed to run threshold repair: {err:?}"),
        })?
    }};
}

pub struct ZcashFrostRepairExtraParams {
    i: u16,
    t: u16,
    helpers: Vec<u16>,
    participant: u16,
    job_id: JobId,
    role_type: RoleType,
    keyshare: FrostKeyShare,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        KBE: KeystoreBackend,
        C: ClientWithApi<B, BE>,
        N: Network,
    > AsyncProtocol for ZcashFrostRepairProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = ZcashFrostRepairExtraParams;
    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_channel: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let debug_logger_post = self.logger.clone();
        let logger = debug_logger_post.clone();
        let protocol_output = Arc::new(tokio::sync::Mutex::new(None));
        let protocol_output_clone = protocol_output.clone();
        let client = self.client.clone();
        let id = self.account_id;
        let network = self.network.clone();

        let (i, helpers, t, participant, keyshare, role_type, mapping) = (
            additional_params.i,
            additional_params.helpers,
            additional_params.t,
            additional_params.participant,
            additional_params.keyshare,
            additional_params.role_type,
            additional_params.user_id_to_account_id_mapping.clone(),
        );

        let role = match role_type {
            RoleType::Tss(role) => role,
            _ => {
                return Err(JobError {
                    reason: "Invalid role type".to_string(),
                })
            }
        };

        let keyshare2 = keyshare.clone();

        Ok(JobBuilder::new()
            .protocol(async move {
                let mut rng = rand::rngs::StdRng::from_entropy();
                let protocol_message_channel =
                    super::util::CloneableUnboundedReceiver::from(protocol_message_channel);

                logger.info(format!(
                    "Starting Signing Protocol with params: i={i}, t={t}"
                ));

                let (
                    signing_tx_to_outbound,
                    signing_rx_async_proto,
                    _broadcast_tx_to_outbound,
                    _broadcast_rx_from_gadget,
                ) = super::util::create_job_manager_to_async_protocol_channel_split::<_, (), _>(
                    protocol_message_channel.clone(),
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    mapping.clone(),
                    id,
                    network.clone(),
                );

                let mut tracer = dfns_cggmp21::progress::PerfProfiler::new();
                let delivery = (signing_rx_async_proto, signing_tx_to_outbound);
                let party = MpcParty::connected(delivery);
                let signature = match role {
                    ThresholdSignatureRoleType::ZcashFrostSecp256k1 => {
                        run_threshold_repair!(
                            Secp256K1Sha256,
                            keyshare,
                            &mut tracer,
                            i,
                            helpers,
                            participant,
                            role,
                            &mut rng,
                            party
                        )
                    }
                    ThresholdSignatureRoleType::ZcashFrostEd25519 => {
                        run_threshold_repair!(
                            Ed25519Sha512,
                            keyshare,
                            &mut tracer,
                            i,
                            helpers,
                            participant,
                            role,
                            &mut rng,
                            party
                        )
                    }
                    ThresholdSignatureRoleType::ZcashFrostP256 => {
                        run_threshold_repair!(
                            P256Sha256,
                            keyshare,
                            &mut tracer,
                            i,
                            helpers,
                            participant,
                            role,
                            &mut rng,
                            party
                        )
                    }
                    ThresholdSignatureRoleType::ZcashFrostRistretto255 => {
                        run_threshold_repair!(
                            Ristretto255Sha512,
                            keyshare,
                            &mut tracer,
                            i,
                            helpers,
                            participant,
                            role,
                            &mut rng,
                            party
                        )
                    }
                    _ => {
                        return Err(JobError {
                            reason: "Invalid role type".to_string(),
                        })
                    }
                };
                let perf_report = tracer.get_report().map_err(|err| JobError {
                    reason: format!("Signing protocol error: {err:?}"),
                })?;
                logger.trace(format!("Signing protocol report: {perf_report}"));
                logger.debug("Finished AsyncProtocol - Signing");
                *protocol_output.lock().await = Some(signature);
                Ok(())
            })
            .post(async move {
                // Submit the protocol output to the blockchain
                if let Some(signature) = protocol_output_clone.lock().await.take() {
                    // TODO: Submit some job result to the blockchain.
                }

                Ok(())
            })
            .build())
    }
}
