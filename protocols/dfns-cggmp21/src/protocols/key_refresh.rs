use async_trait::async_trait;
use dfns_cggmp21::supported_curves::Secp256k1;
use dfns_cggmp21::KeyShare;
use gadget_common::client::{
    AccountId, ClientWithApi, GadgetJobType, JobsApiForGadget, JobsClient,
};
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
use rand::SeedableRng;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::sp_core::keccak_256;
use std::collections::HashMap;
use std::sync::Arc;
use tangle_primitives::jobs::{
    DKGTSSKeyRefreshResult, DigitalSignatureScheme, JobId, JobResult, JobType,
};
use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
use tokio::sync::mpsc::UnboundedReceiver;

pub struct DfnsCGGMP21KeyRefreshProtocol<B: Block, BE, KBE: KeystoreBackend, C, N> {
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
) -> DfnsCGGMP21KeyRefreshProtocol<B, BE, KBE, C, N>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    DfnsCGGMP21KeyRefreshProtocol {
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
    > GadgetProtocol<B, BE, C> for DfnsCGGMP21KeyRefreshProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    fn name(&self) -> String {
        "dfns-cggmp21-key-refresh".to_string()
    }

    async fn create_next_job(
        &self,
        job: JobInitMetadata<B>,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, gadget_common::Error> {
        let job_id = job.job_id;
        let role_type = job.job_type.get_role_type();

        // We can safely make this assumption because we are only creating jobs for phase one
        let JobType::DKGTSSPhaseThree(p3_job) = job.job_type else {
            panic!("Should be valid type")
        };

        let phase1_job = job.phase1_job.expect("Should exist for a phase 2 job");
        let participants = phase1_job.clone().get_participants().expect("Should exist");
        let user_id_to_account_id_mapping = Arc::new(
            participants
                .clone()
                .into_iter()
                .enumerate()
                .map(|r| (r.0 as UserID, r.1))
                .collect(),
        );

        let key = self
            .key_store
            .get_job_result(p3_job.phase_one_id)
            .await
            .map_err(|err| gadget_common::Error::ClientError {
                err: err.to_string(),
            })?
            .ok_or_else(|| gadget_common::Error::ClientError {
                err: format!("No key found for job ID: {job_id:?}"),
            })?;

        let phase_one_id_bytes = p3_job.phase_one_id.to_be_bytes();
        let pregenerated_primes_key =
            keccak_256(&[&b"dfns-cggmp21-keygen-primes"[..], &phase_one_id_bytes[..]].concat());
        let pregenerated_primes = self
            .key_store
            .get(&pregenerated_primes_key)
            .await?
            .ok_or_else(|| gadget_common::Error::ClientError {
                err: format!(
                    "No pregenerated primes found for job ID: {}",
                    p3_job.phase_one_id
                ),
            })?;

        let params = DfnsCGGMP21KeyRefreshExtraParams {
            phase_one_id: p3_job.phase_one_id,
            key,
            pregenerated_primes,
            role_type,
            job_id,
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
        log::error!(target: "dfns_cggmp1", "Error: {error:?}");
    }

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn role_filter(&self, role: RoleType) -> bool {
        matches!(
            role,
            RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1)
        )
    }

    fn phase_filter(&self, job: GadgetJobType) -> bool {
        matches!(job, JobType::DKGTSSPhaseThree(_))
    }

    fn client(&self) -> JobsClient<B, BE, C> {
        self.client.clone()
    }

    fn logger(&self) -> DebugLogger {
        self.logger.clone()
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: None, // Manual polling
            max_active_tasks: crate::constants::keygen_worker::MAX_RUNNING_TASKS,
            max_pending_tasks: crate::constants::keygen_worker::MAX_ENQUEUED_TASKS,
        }
    }
}

pub struct DfnsCGGMP21KeyRefreshExtraParams {
    job_id: JobId,
    phase_one_id: JobId,
    role_type: RoleType,
    key: KeyShare<Secp256k1>,
    pregenerated_primes: dfns_cggmp21::PregeneratedPrimes,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        KBE: KeystoreBackend,
        C: ClientWithApi<B, BE>,
        N: Network,
    > AsyncProtocol for DfnsCGGMP21KeyRefreshProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    type AdditionalParams = DfnsCGGMP21KeyRefreshExtraParams;
    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_channel: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let key_store = self.key_store.clone();
        let protocol_output = Arc::new(tokio::sync::Mutex::new(None));
        let protocol_output_clone = protocol_output.clone();
        let client = self.client.clone();
        let id = self.account_id;
        let logger = self.logger.clone();
        let network = self.network.clone();

        let (mapping, key, pregenerated_primes) = (
            additional_params.user_id_to_account_id_mapping,
            additional_params.key,
            additional_params.pregenerated_primes,
        );
        let i = key.i;
        let n = key.public_shares.len() as u16;
        let t = key.vss_setup.as_ref().map(|x| x.min_signers).unwrap_or(n);

        Ok(JobBuilder::new()
            .protocol(async move {
                let mut rng = rand::rngs::StdRng::from_entropy();
                let protocol_message_channel =
                    super::util::CloneableUnboundedReceiver::from(protocol_message_channel);
                logger.info(format!(
                    "Starting KeyRefresh Protocol with params: i={i}, t={t}, n={n}"
                ));

                let job_id_bytes = additional_params.job_id.to_be_bytes();
                let mix = keccak_256(b"dnfs-cggmp21-keyrefresh");
                let eid_bytes = [&job_id_bytes[..], &mix[..]].concat();
                let eid = dfns_cggmp21::ExecutionId::new(&eid_bytes);

                let (
                    key_refresh_tx_to_outbound,
                    key_refresh_rx_async_proto,
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
                let delivery = (key_refresh_rx_async_proto, key_refresh_tx_to_outbound);
                let party = dfns_cggmp21::round_based::MpcParty::connected(delivery);
                let aux_info = dfns_cggmp21::aux_info_gen(eid, i, n, pregenerated_primes)
                    .set_progress_tracer(&mut tracer)
                    .start(&mut rng, party)
                    .await
                    .map_err(|err| JobError {
                        reason: format!("KeyRefresh protocol error: {err:?}"),
                    })?;

                let key = key.update_aux(aux_info).map_err(|err| JobError {
                    reason: format!("KeyRefresh protocol error: {err:?}"),
                })?;
                let perf_report = tracer.get_report().map_err(|err| JobError {
                    reason: format!("KeyRefresh protocol error: {err:?}"),
                })?;
                logger.trace(format!("KeyRefresh protocol report: {perf_report}"));

                logger.debug("Finished AsyncProtocol - KeyRefresh");

                let job_result = JobResult::DKGPhaseThree(DKGTSSKeyRefreshResult {
                    signature_scheme: DigitalSignatureScheme::Ecdsa,
                });
                *protocol_output.lock().await = Some((key, job_result));
                Ok(())
            })
            .post(async move {
                // TODO: handle protocol blames
                if let Some((local_key, job_result)) = protocol_output_clone.lock().await.take() {
                    // Update the local key share, of the phase one.
                    key_store
                        .set_job_result(additional_params.phase_one_id, local_key)
                        .await
                        .map_err(|err| JobError {
                            reason: format!("Failed to store key: {err:?}"),
                        })?;

                    client
                        .submit_job_result(
                            additional_params.role_type,
                            additional_params.job_id,
                            job_result,
                        )
                        .await
                        .map_err(|err| JobError {
                            reason: format!("Failed to submit job result: {err:?}"),
                        })?;
                }

                Ok(())
            })
            .build())
    }
}
