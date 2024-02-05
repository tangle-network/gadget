use crate::protocol::state_machine::payloads::RoundPayload;
use crate::protocol::state_machine::BlsStateMachine;
use async_trait::async_trait;
use gadget_common::client::{AccountId, ClientWithApi, JobsClient, PalletSubmitter};
use gadget_common::config::{DebugLogger, GadgetProtocol, JobsApi, Network, ProvideRuntimeApi};
use gadget_common::gadget::message::{GadgetProtocolMessage, UserID};
use gadget_common::gadget::work_manager::WorkManager;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::keystore::{GenericKeyStore, KeystoreBackend};
use gadget_common::protocol::AsyncProtocol;
use gadget_common::sp_core::keccak_256;
use gadget_common::{
    Backend, Block, BlockImportNotification, BuiltExecutableJobWrapper, Error, JobBuilder,
    JobError, ProtocolWorkManager, WorkManagerInterface,
};
use itertools::Itertools;
use round_based::{AsyncProtocol as RoundsBasedAsyncProtocol, Msg};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tangle_primitives::jobs::{DKGTSSKeySubmissionResult, DigitalSignatureType, JobId, JobResult};
use tangle_primitives::roles::{RoleType, ThresholdSignatureRoleType};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

pub struct BlsKeygenProtocol<
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
> where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    pub jobs_client: JobsClient<B, BE, C>,
    pub account_id: AccountId,
    pub logger: DebugLogger,
    pub network: N,
    pub pallet_tx: Arc<dyn PalletSubmitter>,
    pub keystore: GenericKeyStore<KBE, gadget_common::sp_core::ecdsa::Pair>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, N: Network, KBE: KeystoreBackend>
    GadgetProtocol<B, BE, C> for BlsKeygenProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    async fn create_next_job(
        &self,
        job: JobInitMetadata,
    ) -> Result<<Self as AsyncProtocol>::AdditionalParams, Error> {
        let job_id = job.job_id;
        let p1_job = job.job_type;
        let threshold = p1_job.clone().get_threshold().expect("Should exist") as u16;
        let role_type = p1_job.get_role_type();
        let participants = p1_job.get_participants().expect("Should exist");
        let user_id_to_account_id_mapping = Arc::new(
            participants
                .clone()
                .into_iter()
                .enumerate()
                .map(|r| ((r.0 + 1) as UserID, r.1))
                .collect(),
        );

        let additional_params = BlsKeygenAdditionalParams {
            i: (participants
                .iter()
                .position(|p| p == &self.account_id)
                .expect("Should exist")
                + 1) as u16,
            t: threshold,
            n: participants.len() as u16,
            role_type,
            job_id,
            user_id_to_account_id_mapping,
        };

        Ok(additional_params)
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WorkManager>,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn process_error(&self, _error: Error, _job_manager: &ProtocolWorkManager<WorkManager>) {}

    fn account_id(&self) -> &AccountId {
        &self.account_id
    }

    fn role_type(&self) -> RoleType {
        RoleType::Tss(ThresholdSignatureRoleType::GennaroDKGBls381)
    }

    fn is_phase_one(&self) -> bool {
        true
    }

    fn client(&self) -> &JobsClient<B, BE, C> {
        &self.jobs_client
    }

    fn logger(&self) -> &DebugLogger {
        &self.logger
    }
}

#[derive(Clone)]
pub struct BlsKeygenAdditionalParams {
    i: u16,
    t: u16,
    n: u16,
    role_type: RoleType,
    job_id: JobId,
    user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, N: Network, KBE: KeystoreBackend>
    AsyncProtocol for BlsKeygenProtocol<B, BE, C, N, KBE>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApi<B, AccountId>,
{
    type AdditionalParams = BlsKeygenAdditionalParams;

    async fn generate_protocol_from(
        &self,
        associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let network = self.network.clone();
        let result = Arc::new(tokio::sync::Mutex::new(None));
        let result_clone = result.clone();
        let keystore = self.keystore.clone();
        let keystore_clone = keystore.clone();
        let job_id = additional_params.job_id;
        let pallet_tx = self.pallet_tx.clone();
        let role_type = additional_params.role_type;
        let (i, t, n) = (
            additional_params.i,
            additional_params.t,
            additional_params.n,
        );
        let logger = self.logger.clone();
        let user_id_to_account_id = additional_params.user_id_to_account_id_mapping.clone();

        Ok(JobBuilder::new()
            .protocol(async move {
                logger.info("Beginning BlsKeygenProtocol ...");
                let state_machine =
                    BlsStateMachine::new(i, t, n, logger.clone()).map_err(|err| JobError {
                        reason: err.to_string(),
                    })?;

                let (tx0, rx0, mut tx1, mut rx1) =
                    gadget_common::channels::create_job_manager_to_async_protocol_channel_split::<
                        _,
                        Msg<RoundPayload>,
                        Msg<RoundPayload>,
                    >(
                        protocol_message_rx,
                        associated_block_id,
                        associated_retry_id,
                        associated_session_id,
                        associated_task_id,
                        user_id_to_account_id.clone(),
                        network.clone(),
                    );

                let me = RoundsBasedAsyncProtocol::new(state_machine, rx0, tx0)
                    .run()
                    .await
                    .map_err(|err| JobError {
                        reason: err.to_string(),
                    })?;

                logger.info("Completed BlsKeygenProtocol ...");

                let public_key = me
                    .get_public_key()
                    .ok_or_else(|| JobError {
                        reason: "Failed to get public key".to_string(),
                    })?
                    .to_uncompressed()
                    .to_vec();

                let sk = me.get_secret_share().ok_or_else(|| JobError {
                    reason: "Failed to get secret share".to_string(),
                })?;

                let share =
                    blst::min_pk::SecretKey::from_bytes(&sk.to_be_bytes()).map_err(|e| {
                        JobError {
                            reason: format!("Failed to create secret key: {e:?}"),
                        }
                    })?;

                let pk_share = share.sk_to_pk();

                let job_result = handle_public_key_broadcast(
                    &keystore,
                    pk_share,
                    additional_params.i,
                    additional_params.t,
                    &mut tx1,
                    &mut rx1,
                    &user_id_to_account_id,
                )
                .await?;

                logger.info("Finished public key broadcast ...");

                *result.lock().await = Some((job_result, me));

                Ok(())
            })
            .post(async move {
                if let Some((job_result, secret)) = result_clone.lock().await.take() {
                    keystore_clone
                        .set(job_id, secret)
                        .await
                        .map_err(|err| JobError {
                            reason: err.to_string(),
                        })?;

                    pallet_tx
                        .submit_job_result(role_type, job_id, job_result)
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

async fn handle_public_key_broadcast<KBE: KeystoreBackend>(
    key_store: &GenericKeyStore<KBE, gadget_common::sp_core::ecdsa::Pair>,
    public_key_share: blst::min_pk::PublicKey,
    i: u16,
    t: u16,
    tx: &mut UnboundedSender<Msg<RoundPayload>>,
    rx: &mut UnboundedReceiver<Msg<RoundPayload>>,
    user_id_to_account_id_mapping: &Arc<HashMap<UserID, AccountId>>,
) -> Result<JobResult, JobError> {
    let mut received_pk_shares = BTreeMap::new();
    received_pk_shares.insert(i, public_key_share.clone());

    let broadcast_message =
        RoundPayload::PublicKeyGossipRound1(public_key_share.serialize().to_vec());
    tx.send(Msg {
        sender: i,
        receiver: None,
        body: broadcast_message,
    })
    .map_err(|err| JobError {
        reason: err.to_string(),
    })?;

    // Receive t signatures
    let mut count = 0;
    while count < t {
        let payload = rx.recv().await.ok_or_else(|| JobError {
            reason: "Failed to receive message".to_string(),
        })?;
        match payload.body {
            RoundPayload::PublicKeyGossipRound1(pk_share) => {
                match blst::min_pk::PublicKey::deserialize(&pk_share) {
                    Ok(pk) => {
                        received_pk_shares.insert(payload.sender, pk);
                    }
                    Err(e) => {
                        log::warn!(target: "gadget", "Failed to deserialize public key: {e:?}");
                    }
                }
            }
            _ => {
                return Err(JobError {
                    reason: "Unexpected payload".to_string(),
                })
            }
        }
        count += 1;
    }

    let pk_shares = received_pk_shares
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1)
        .collect::<Vec<_>>();

    let pk_agg =
        blst::min_pk::AggregatePublicKey::aggregate(&pk_shares.iter().collect::<Vec<_>>(), true)
            .map_err(|e| JobError {
                reason: format!("Failed to aggregate public keys: {e:?}"),
            })?;

    // Now, sign this aggregated public key
    let key_hashed = keccak_256(&pk_agg.to_public_key().serialize());
    let signature = key_store.pair().sign_prehashed(&key_hashed).0.to_vec();

    let mut received_signatures = BTreeMap::new();
    received_signatures.insert(i, signature.clone());

    // Now, broadcast the signature and collect them
    let broadcast_message = RoundPayload::PublicKeyGossipRound2(signature.clone());
    tx.send(Msg {
        sender: i,
        receiver: None,
        body: broadcast_message,
    })
    .map_err(|err| JobError {
        reason: err.to_string(),
    })?;

    let mut count = 0;
    while count < t {
        let payload = rx.recv().await.ok_or_else(|| JobError {
            reason: "Failed to receive message".to_string(),
        })?;
        match payload.body {
            RoundPayload::PublicKeyGossipRound2(sig) => {
                received_signatures.insert(payload.sender, sig);
            }
            _ => {
                return Err(JobError {
                    reason: "Unexpected payload".to_string(),
                })
            }
        }
        count += 1;
    }

    let participants = user_id_to_account_id_mapping
        .iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1 .0.to_vec())
        .collect();

    let signatures = received_signatures
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1)
        .collect();

    Ok(JobResult::DKGPhaseOne(DKGTSSKeySubmissionResult {
        signature_type: DigitalSignatureType::Bls381,
        key: pk_agg.to_public_key().serialize().to_vec(), // The pallet is assuming this is the signing key, but it's not
        participants,
        signatures,
        threshold: t as u8,
    }))
}
