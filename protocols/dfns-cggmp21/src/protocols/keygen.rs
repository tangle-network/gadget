use crate::DfnsCGGMP21ProtocolConfig;
use async_trait::async_trait;
use dfns_cggmp21::supported_curves::Secp256k1;
use dfns_cggmp21::KeyShare;
use frame_support::Hashable;
use futures::StreamExt;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient};
use gadget_common::debug_logger::DebugLogger;
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::{Job, TangleGadgetProtocol, WorkManagerConfig};
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use gadget_common::protocol::AsyncProtocol;
use gadget_common::{Block, BlockImportNotification, FinalityNotification};
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use itertools::Itertools;
use pallet_jobs_rpc_runtime_api::JobsApi;
use rand::SeedableRng;
use sc_client_api::Backend;
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::sp_core::keccak_256;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tangle_primitives::jobs::{
    DKGTSSKeySubmissionResult, DigitalSignatureType, JobId, JobResult, JobType,
};
use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};

use super::util::PublicKeyGossipMessage;

pub struct DfnsCGGMP21KeygenProtocol<B: Block, BE, KBE: KeystoreBackend, C, N> {
    client: JobsClient<B, BE, C>,
    key_store: ECDSAKeyStore<KBE>,
    network: N,
    logger: DebugLogger,
    account_id: AccountId,
}

pub async fn create_protocol<B, BE, KBE, C, N>(
    config: &DfnsCGGMP21ProtocolConfig,
    client: JobsClient<B, BE, C>,
    network: N,
    logger: DebugLogger,
    key_store: ECDSAKeyStore<KBE>,
) -> DfnsCGGMP21KeygenProtocol<B, BE, KBE, C, N>
where
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    KBE: KeystoreBackend,
    N: Network,
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    DfnsCGGMP21KeygenProtocol {
        client,
        network,
        key_store,
        logger,
        account_id: config.account_id,
    }
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        C: ClientWithApi<B, BE>,
        KBE: KeystoreBackend,
        N: Network,
    > TangleGadgetProtocol<B> for DfnsCGGMP21KeygenProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    async fn get_next_jobs(
        &self,
        notification: &FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<Option<Vec<Job>>, gadget_common::Error> {
        self.logger.info(format!("At finality notification {now}"));
        let jobs = self
            .client
            .query_jobs_by_validator(notification.hash, self.account_id)
            .await?;

        let mut ret = vec![];

        for job in jobs {
            let job_id = job.job_id;
            let role_type = job.job_type.get_role_type();
            if let JobType::DKGTSSPhaseOne(p1_job) = job.job_type {
                if p1_job.role_type != ThresholdSignatureRoleType::TssCGGMP {
                    continue;
                }
                let participants = p1_job.participants;
                let threshold = p1_job.threshold;
                if participants.contains(&self.account_id) {
                    let task_id = job.job_id.to_be_bytes();
                    let task_id = keccak_256(&task_id);
                    let session_id = 0; // We are not interested in sessions for the ECDSA protocol
                    let retry_id = job_manager
                        .latest_retry_id(&task_id)
                        .map(|r| r + 1)
                        .unwrap_or(0);

                    let user_id_to_account_id_mapping = Arc::new(
                        participants
                            .clone()
                            .into_iter()
                            .enumerate()
                            .map(|r| (r.0 as UserID, r.1))
                            .collect(),
                    );

                    let additional_params = DfnsCGGMP21KeygenExtraParams {
                        i: participants
                            .iter()
                            .position(|p| p == &self.account_id)
                            .expect("Should exist") as u16,
                        t: threshold as u16,
                        n: participants.len() as u16,
                        role_type,
                        job_id,
                        user_id_to_account_id_mapping,
                    };

                    let job = self
                        .create(session_id, now, retry_id, task_id, additional_params)
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

    fn logger(&self) -> &DebugLogger {
        &self.logger
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: None, // Manual polling
            max_active_tasks: crate::constants::keygen_worker::MAX_RUNNING_TASKS,
            max_pending_tasks: crate::constants::keygen_worker::MAX_ENQUEUED_TASKS,
        }
    }
}

pub struct DfnsCGGMP21KeygenExtraParams {
    i: u16,
    t: u16,
    n: u16,
    job_id: JobId,
    role_type: RoleType,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

#[async_trait]
impl<
        B: Block,
        BE: Backend<B> + 'static,
        KBE: KeystoreBackend,
        C: ClientWithApi<B, BE>,
        N: Network,
    > AsyncProtocol for DfnsCGGMP21KeygenProtocol<B, BE, KBE, C, N>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = DfnsCGGMP21KeygenExtraParams;
    async fn generate_protocol_from(
        &self,
        associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
        protocol_message_channel: tokio::sync::broadcast::Sender<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let key_store = self.key_store.clone();
        let protocol_output = Arc::new(tokio::sync::Mutex::new(None));
        let protocol_output_clone = protocol_output.clone();
        let client = self.client.clone();
        let id = self.account_id;
        let logger = self.logger.clone();
        let network = self.network.clone();

        let (i, t, n, mapping) = (
            additional_params.i,
            additional_params.t,
            additional_params.n,
            additional_params.user_id_to_account_id_mapping,
        );

        Ok(JobBuilder::new()
            .protocol(async move {
                let mut rng = rand::rngs::StdRng::from_entropy();
                logger.info(format!(
                    "Starting Keygen Protocol with params: i={i}, t={t}, n={n}"
                ));

                let job_id_bytes = additional_params.job_id.to_be_bytes();
                let mix = keccak_256(b"dnfs-cggmp21-keygen");
                let eid_bytes = [&job_id_bytes[..], &mix[..]].concat();
                let eid = dfns_cggmp21::ExecutionId::new(&eid_bytes);
                let mix = keccak_256(b"dnfs-cggmp21-keygen-aux");
                let aux_eid_bytes = [&job_id_bytes[..], &mix[..]].concat();
                let aux_eid = dfns_cggmp21::ExecutionId::new(&aux_eid_bytes);
                let (
                    keygen_tx_to_outbound,
                    keygen_rx_async_proto,
                    _broadcast_tx_to_outbound,
                    _broadcast_rx_from_gadget,
                ) = super::util::create_job_manager_to_async_protocol_channel_split::<_, (), _>(
                    protocol_message_channel.subscribe(),
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    mapping.clone(),
                    id,
                    network.clone(),
                );
                let delivery = (keygen_rx_async_proto, keygen_tx_to_outbound);
                let party = dfns_cggmp21::round_based::MpcParty::connected(delivery);
                let incomplete_key_share = dfns_cggmp21::keygen::<Secp256k1>(eid, i, n)
                    .set_threshold(t)
                    .start(&mut rng, party)
                    .await
                    .map_err(|err| JobError {
                        reason: format!("Keygen protocol error: {err:?}"),
                    })?;

                logger.debug("Finished AsyncProtocol - Incomplete Keygen");

                let (
                    keygen_tx_to_outbound,
                    keygen_rx_async_proto,
                    broadcast_tx_to_outbound,
                    broadcast_rx_from_gadget,
                ) = super::util::create_job_manager_to_async_protocol_channel_split(
                    protocol_message_channel.subscribe(),
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    mapping,
                    id,
                    network,
                );
                let pregenerated_primes = dfns_cggmp21::PregeneratedPrimes::generate(&mut rng);
                let delivery = (keygen_rx_async_proto, keygen_tx_to_outbound);
                let party = dfns_cggmp21::round_based::MpcParty::connected(delivery);
                let aux_info = dfns_cggmp21::aux_info_gen(aux_eid, i, n, pregenerated_primes)
                    .start(&mut rng, party)
                    .await
                    .map_err(|err| JobError {
                        reason: format!("Aux info protocol error: {err:?}"),
                    })?;
                logger.debug("Finished AsyncProtocol - Aux Info");

                let key_share = dfns_cggmp21::KeyShare::make(incomplete_key_share, aux_info)
                    .map_err(|err| JobError {
                        reason: format!("Key share error: {err:?}"),
                    })?;

                logger.debug("Finished AsyncProtocol - Keygen");

                let job_result = handle_public_key_gossip(
                    &logger,
                    &key_share,
                    t,
                    i,
                    id,
                    broadcast_tx_to_outbound,
                    broadcast_rx_from_gadget,
                )
                .await?;

                *protocol_output.lock().await = Some((key_share, job_result));
                Ok(())
            })
            .post(async move {
                // TODO: handle protocol blames
                // Store the keys locally, as well as submitting them to the blockchain
                if let Some((local_key, job_result)) = protocol_output_clone.lock().await.take() {
                    key_store
                        .set(additional_params.job_id, local_key)
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

async fn handle_public_key_gossip(
    logger: &DebugLogger,
    local_key: &KeyShare<Secp256k1>,
    threshold: u16,
    i: u16,
    my_id: AccountId,
    broadcast_tx_to_outbound: futures::channel::mpsc::UnboundedSender<PublicKeyGossipMessage>,
    mut broadcast_rx_from_gadget: futures::channel::mpsc::UnboundedReceiver<PublicKeyGossipMessage>,
) -> Result<JobResult, JobError> {
    let serialized_public_key = local_key.shared_public_key().to_bytes(true).to_vec();
    // Note: the signature is also empty in the DKG implementation
    let signature = vec![];

    let mut received_keys = BTreeMap::new();
    received_keys.insert(i, signature.clone());
    let mut received_participants = BTreeMap::new();
    received_participants.insert(i, my_id);

    broadcast_tx_to_outbound
        .unbounded_send(PublicKeyGossipMessage {
            from: i as _,
            to: None,
            signature,
            id: my_id,
        })
        .map_err(|err| JobError {
            reason: format!("Failed to send public key: {err:?}"),
        })?;

    for idx in 0..threshold {
        let message = broadcast_rx_from_gadget
            .next()
            .await
            .ok_or_else(|| JobError {
                reason: "Failed to receive public key".to_string(),
            })?;

        let from = message.from;
        logger.debug(format!(
            "Received public key from {from} | {}/{threshold} received",
            idx + 1
        ));

        if received_keys.contains_key(&(from as u16)) {
            logger.warn("Received duplicate key");
            continue;
        }

        received_keys.insert(from as u16, message.signature);
        received_participants.insert(from as u16, message.id);
    }

    // Order and collect the map to ensure symmetric submission to blockchain
    let signatures = received_keys
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1)
        .collect();

    let participants = received_participants
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1.identity())
        .collect();

    Ok(JobResult::DKGPhaseOne(DKGTSSKeySubmissionResult {
        signature_type: DigitalSignatureType::Ecdsa,
        key: serialized_public_key,
        participants,
        signatures,
        threshold: threshold as _,
    }))
}
