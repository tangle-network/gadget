use crate::protocol::state_machine::payloads::RoundPayload;
use crate::protocol::state_machine::BlsStateMachine;
use gadget_common::gadget::message::UserID;
use gadget_common::keystore::{ECDSAKeyStore, KeystoreBackend};
use gadget_common::prelude::*;
use gadget_common::sp_core::keccak_256;
use itertools::Itertools;
use round_based::Msg;
use std::collections::{BTreeMap, HashMap};
use tangle_primitives::jobs::{
    DKGTSSKeySubmissionResult, DigitalSignatureScheme, JobId, JobResult,
};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone)]
pub struct BlsKeygenAdditionalParams {
    pub i: u16,
    pub t: u16,
    pub n: u16,
    pub role_type: RoleType,
    pub job_id: JobId,
    pub user_id_to_account_id_mapping: Arc<HashMap<UserID, AccountId>>,
}

pub async fn create_next_job<
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
>(
    config: &crate::BlsKeygenProtocol<B, BE, C, N, KBE>,
    job: JobInitMetadata<B>,
) -> Result<BlsKeygenAdditionalParams, Error>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
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
            .position(|p| p == &config.account_id)
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

pub async fn generate_protocol_from<
    B: Block,
    BE: Backend<B>,
    C: ClientWithApi<B, BE>,
    N: Network,
    KBE: KeystoreBackend,
>(
    config: &crate::BlsKeygenProtocol<B, BE, C, N, KBE>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
    protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
    additional_params: BlsKeygenAdditionalParams,
) -> Result<BuiltExecutableJobWrapper, JobError>
where
    <C as ProvideRuntimeApi<B>>::Api: JobsApiForGadget<B>,
{
    let network = config.network.clone();
    let result = Arc::new(tokio::sync::Mutex::new(None));
    let result_clone = result.clone();
    let keystore = config.key_store.clone();
    let keystore_clone = keystore.clone();
    let job_id = additional_params.job_id;
    let pallet_tx = config.pallet_tx.clone();
    let role_type = additional_params.role_type;
    let (i, t, n) = (
        additional_params.i,
        additional_params.t,
        additional_params.n,
    );
    let logger = config.logger.clone();
    let user_id_to_account_id = additional_params.user_id_to_account_id_mapping.clone();

    Ok(JobBuilder::new()
        .protocol(async move {
            logger.info("Beginning BlsKeygenProtocol ...");
            let state_machine =
                BlsStateMachine::new(i, t, n, logger.clone()).map_err(|err| JobError {
                    reason: err.to_string(),
                })?;

            let (tx0, rx0, tx1, mut rx1) =
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

            let me = round_based::AsyncProtocol::new(state_machine, rx0, tx0)
                .run()
                .await
                .map_err(|err| JobError {
                    reason: err.to_string(),
                })?;

            logger.info("Completed BlsKeygenProtocol ...");

            let sk = me.get_secret_share().ok_or_else(|| JobError {
                reason: "Failed to get secret share".to_string(),
            })?;

            let share =
                snowbridge_milagro_bls::SecretKey::from_bytes(&sk.to_be_bytes()).map_err(|e| {
                    JobError {
                        reason: format!("Failed to create secret key: {e:?}"),
                    }
                })?;

            let pk_share = snowbridge_milagro_bls::PublicKey::from_secret_key(&share);

            let job_result = crate::protocol::keygen::handle_public_key_broadcast(
                &keystore,
                pk_share,
                additional_params.i,
                additional_params.t,
                &tx1,
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
                let key = keccak_256(&job_id.to_be_bytes());
                keystore_clone
                    .set(&key, secret)
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

pub(crate) async fn handle_public_key_broadcast<KBE: KeystoreBackend>(
    key_store: &ECDSAKeyStore<KBE>,
    public_key_share: snowbridge_milagro_bls::PublicKey,
    i: u16,
    t: u16,
    tx: &UnboundedSender<Msg<RoundPayload>>,
    rx: &mut UnboundedReceiver<Msg<RoundPayload>>,
    user_id_to_account_id_mapping: &Arc<HashMap<UserID, AccountId>>,
) -> Result<GadgetJobResult, JobError> {
    let mut received_pk_shares = BTreeMap::new();
    let mut received_signatures = BTreeMap::new();
    received_pk_shares.insert(i, public_key_share.clone());

    let broadcast_message =
        RoundPayload::PublicKeyGossipRound1(public_key_share.as_uncompressed_bytes().to_vec());
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
                match snowbridge_milagro_bls::PublicKey::from_uncompressed_bytes(&pk_share) {
                    Ok(pk) => {
                        received_pk_shares.insert(payload.sender, pk);
                    }
                    Err(e) => {
                        log::warn!(target: "gadget", "Failed to deserialize public key: {e:?}");
                    }
                }
            }

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

    let pk_shares = received_pk_shares
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1)
        .collect::<Vec<_>>();

    let pk_agg = snowbridge_milagro_bls::AggregatePublicKey::aggregate(
        &pk_shares.iter().collect::<Vec<_>>(),
    )
    .map_err(|e| JobError {
        reason: format!("Failed to aggregate public keys: {e:?}"),
    })?;

    let uncompressed_public_key = &mut [0u8; 97];
    pk_agg.point.to_bytes(uncompressed_public_key, false);
    // Now, sign this aggregated public key
    let key_hashed = keccak_256(&uncompressed_public_key[1..]);
    let signature = key_store.pair().sign_prehashed(&key_hashed).0.to_vec();

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

    let mut count = received_signatures.len();
    while count < t as usize {
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
        .map(|r| r.1 .0.to_vec().try_into().unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let signatures = received_signatures
        .into_iter()
        .sorted_by_key(|x| x.0)
        .map(|r| r.1.try_into().unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let key = uncompressed_public_key[1..].to_vec();

    Ok(JobResult::DKGPhaseOne(DKGTSSKeySubmissionResult {
        signature_scheme: DigitalSignatureScheme::Bls381,
        key: key.try_into().unwrap(),
        participants,
        signatures,
        threshold: t as u8,
    }))
}
