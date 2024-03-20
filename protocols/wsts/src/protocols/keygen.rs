use crate::protocols::util::{
    generate_party_key_ids, validate_parameters, FrostMessage, FrostState,
};
use futures::{SinkExt, StreamExt};
use gadget_common::client::ClientWithApi;
use gadget_common::config::Network;
use gadget_common::gadget::message::UserID;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::keystore::KeystoreBackend;
use gadget_common::prelude::jobs::tss::{DKGTSSKeySubmissionResult, DigitalSignatureScheme};
use gadget_common::prelude::jobs::JobResult;
use gadget_common::prelude::roles::tss::ThresholdSignatureRoleType;
use gadget_common::prelude::roles::RoleType;
use gadget_common::prelude::JobError;
use gadget_common::prelude::{DebugLogger, GadgetProtocolMessage, WorkManager};
use gadget_common::{
    BuiltExecutableJobWrapper, JobBuilder, ProtocolWorkManager, WorkManagerInterface,
};
use hashbrown::HashMap;
use rand::{CryptoRng, RngCore};
use sp_core::ecdsa;
use std::sync::Arc;
use tangle_primitives::jobs::JobId;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use wsts::common::PolyCommitment;
use wsts::curve::scalar::Scalar;
use wsts::v2::Party;

#[derive(Clone)]
pub struct WstsKeygenExtraParams {
    job_id: JobId,
    n: u32,
    i: u32,
    k: u32,
    t: u32,
    user_id_mapping: Arc<std::collections::HashMap<UserID, ecdsa::Public>>,
    my_id: ecdsa::Public,
}

pub async fn create_next_job<KBE: KeystoreBackend, C: ClientWithApi, N: Network>(
    config: &crate::WstsKeygenProtocol<C, N, KBE>,
    job: JobInitMetadata,
    _work_manager: &ProtocolWorkManager<WorkManager>,
) -> Result<WstsKeygenExtraParams, gadget_common::Error> {
}

pub async fn generate_protocol_from<KBE: KeystoreBackend, C: ClientWithApi, N: Network>(
    config: &crate::WstsKeygenProtocol<C, N, KBE>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
    protocol_message_channel: UnboundedReceiver<GadgetProtocolMessage>,
    additional_params: WstsKeygenExtraParams,
) -> Result<BuiltExecutableJobWrapper, JobError> {
    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();
    let logger = config.logger.clone();
    let network = config.clone();
    let keystore = config.key_store.clone();
    let client = config.pallet_tx.clone();
    let WstsKeygenExtraParams {
        job_id,
        n,
        i,
        k,
        t,
        user_id_mapping,
        my_id,
    } = additional_params;

    Ok(JobBuilder::new()
        .protocol(async move {
            let (tx0, rx0, tx1, rx1) =
                gadget_common::channels::create_job_manager_to_async_protocol_channel_split::<
                    _,
                    FrostMessage,
                    FrostMessage,
                >(
                    protocol_message_channel,
                    associated_block_id,
                    associated_retry_id,
                    associated_session_id,
                    associated_task_id,
                    user_id_mapping,
                    my_id,
                    network,
                    logger.clone(),
                );
            let output = protocol(n, i, k, t, tx0, rx0, tx1, rx1, &logger).await?;
            result.lock().await.replace(output);
        })
        .post(async move {
            if let Some(state) = result_clone.lock().await.take() {
                let public_key = state.public_key.clone();
                keystore
                    .set_job_result(job_id, state)
                    .await
                    .map_err(|err| JobError {
                        reason: err.to_string(),
                    })?;

                let job_result_for_pallet = JobResult::DKGPhaseOne(DKGTSSKeySubmissionResult {
                    signature_scheme: DigitalSignatureScheme::EcdsaSecp256k1,
                    key: BoundedVec(),
                    participants: BoundedVec(),
                    signatures: BoundedVec(),
                    threshold: t,
                    __subxt_unused_type_params: Default::default(),
                });

                client
                    .submit_job_result(RoleType::Tss(ThresholdSignatureRoleType::Wsts))
                    .await
                    .map_err(|err| JobError {
                        reason: err.to_string(),
                    })?;
            }
        })
        .build())
}

/// `party_id`: Should be in the range [0, n). For the DKG, should be our index in the best
/// authorities starting from 0.
///
/// Returns the state of the party after the protocol has finished. This should be saved to the keystore and
/// later used for signing
#[allow(clippy::too_many_arguments)]
pub async fn protocol(
    n: u32,
    party_id: u32,
    k: u32,
    t: u32,
    tx_to_network: futures::channel::mpsc::UnboundedSender<FrostMessage>,
    rx_from_network: futures::channel::mpsc::UnboundedReceiver<std::io::Result<FrostMessage>>,
    mut tx_to_network_broadcast: tokio::sync::mpsc::UnboundedSender<FrostMessage>,
    rx_from_network_broadcast: tokio::sync::mpsc::UnboundedReceiver<FrostMessage>,
    logger: &DebugLogger,
) -> Result<FrostState, JobError> {
    validate_parameters(n, k, t)?;

    let mut rng = rand::rngs::OsRng;
    let key_ids = generate_party_key_ids(n, k);
    let our_key_ids = key_ids.get(party_id as usize).ok_or_else(|| JobError {
        reason: "Bad party_id".to_string(),
    })?;

    let mut party = Party::new(party_id, our_key_ids, n, k, t, &mut rng);
    let public_key = run_dkg(
        &mut party,
        &mut rng,
        n as usize,
        tx_to_network,
        rx_from_network,
        logger,
    )
    .await?;

    // Gossip the public key
    let pkey_message = FrostMessage::PublicKeyBroadcast {
        party_id,
        public_key: public_key.clone(),
    };

    let party = party.save();

    // Gossip the public key
    tx_to_network_broadcast
        .send(pkey_message)
        .await
        .map_err(|err| JobError {
            reason: format!("Error sending FROST message: {err:?}"),
        })?;

    // TODO: Handle verification of gossiped public keys and save the hashmap of the public key
    let frost_state = FrostState { public_key, party };

    Ok(frost_state)
}

pub async fn run_dkg<RNG: RngCore + CryptoRng>(
    signer: &mut Party,
    rng: &mut RNG,
    n_signers: usize,
    mut tx_to_network: futures::channel::mpsc::UnboundedSender<FrostMessage>,
    mut rx_from_network: futures::channel::mpsc::UnboundedReceiver<std::io::Result<FrostMessage>>,
    logger: &DebugLogger,
) -> Result<HashMap<u32, PolyCommitment>, JobError> {
    // Broadcast our party_id, shares, and key_ids to each other
    let party_id = signer.party_id;
    let shares: HashMap<u32, Scalar> = signer.get_shares().into_iter().collect();
    let key_ids = signer.key_ids.clone();
    let poly_commitment = signer.get_poly_commitment(rng);
    let message = FrostMessage::Keygen {
        party_id,
        shares: shares.clone(),
        key_ids: key_ids.clone(),
        poly_commitment: poly_commitment.clone(),
    };

    // Send the message
    tx_to_network.send(message).await.map_err(|err| JobError {
        reason: format!("Error sending FROST message: {err:?}"),
    })?;

    let mut received_shares = HashMap::new();
    let mut received_key_ids = HashMap::new();
    let mut received_poly_commitments = HashMap::new();
    // insert our own shared into the received map
    received_shares.insert(party_id, shares);
    received_key_ids.insert(party_id, key_ids);
    received_poly_commitments.insert(party_id, poly_commitment);

    // Wait for n_signers to send their messages to us
    while received_shares.len() < n_signers {
        match rx_from_network.next().await {
            Some(FrostMessage::Keygen {
                party_id,
                shares,
                key_ids,
                poly_commitment,
            }) => {
                if party_id != signer.party_id {
                    received_shares.insert(party_id, shares);
                    received_key_ids.insert(party_id, key_ids);
                    received_poly_commitments.insert(party_id, poly_commitment);
                }
            }

            Some(evt) => logger.warn(format!("Received unexpected FROST event: {evt:?}")),

            None => {
                return Err(JobError {
                    reason: "NetListen connection died".to_string(),
                })
            }
        }
    }

    // Generate the party_shares: for each key id we own, we take our received key share at that
    // index
    let party_shares = signer
        .key_ids
        .iter()
        .copied()
        .map(|key_id| {
            let mut key_shares = HashMap::new();

            for (id, shares) in &received_shares {
                key_shares.insert(*id, shares[&key_id]);
            }

            (key_id, key_shares.into_iter().collect())
        })
        .collect();
    signer
        .compute_secret(&party_shares, &received_poly_commitments)
        .map_err(|err| JobError {
            reason: err.to_string(),
        })?;

    Ok(received_poly_commitments)
}
