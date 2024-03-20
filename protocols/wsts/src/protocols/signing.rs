use crate::protocols::util::{FrostMessage, FrostState};
use futures::{SinkExt, StreamExt};
use gadget_common::client::ClientWithApi;
use gadget_common::config::Network;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::keystore::KeystoreBackend;
use gadget_common::prelude::{DebugLogger, GadgetProtocolMessage, WorkManager};
use gadget_common::{
    BuiltExecutableJobWrapper, JobError, ProtocolWorkManager, WorkManagerInterface,
};
use hashbrown::HashMap;
use itertools::Itertools;
use rand::{CryptoRng, RngCore};
use tokio::sync::mpsc::UnboundedReceiver;
use wsts::common::Signature;
use wsts::traits::Aggregator;
use wsts::v2::Party;

#[derive(Clone)]
pub struct WstsSigningExtraParams {}

pub async fn create_next_job<KBE: KeystoreBackend, C: ClientWithApi, N: Network>(
    config: &crate::WstsSigningProtocol<C, N, KBE>,
    job: JobInitMetadata,
    _work_manager: &ProtocolWorkManager<WorkManager>,
) -> Result<WstsSigningExtraParams, gadget_common::Error> {
}

pub async fn generate_protocol_from<KBE: KeystoreBackend, C: ClientWithApi, N: Network>(
    config: &crate::WstsSigningProtocol<C, N, KBE>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
    protocol_message_channel: UnboundedReceiver<GadgetProtocolMessage>,
    additional_params: WstsSigningExtraParams,
) -> Result<BuiltExecutableJobWrapper, JobError> {
}

/// `threshold`: Should be the number of participants in this round, since we stop looking for
/// messages after finding the first `t` messages
#[allow(clippy::too_many_arguments)]
pub async fn run_signing<RNG: RngCore + CryptoRng>(
    state: FrostState,
    rng: &mut RNG,
    msg: &[u8],
    num_keys: u32,
    threshold: u32,
    mut tx_to_network: futures::channel::mpsc::UnboundedSender<FrostMessage>,
    mut rx_from_network: futures::channel::mpsc::UnboundedReceiver<FrostMessage>,
    mut tx_to_network_final: futures::channel::mpsc::UnboundedSender<FrostMessage>,
    mut rx_from_network_final: futures::channel::mpsc::UnboundedReceiver<FrostMessage>,
    logger: &DebugLogger,
) -> Result<Signature, JobError> {
    let mut signer = Party::load(&state.party);
    let public_key = state.public_key;
    // Broadcast the party_id, key_ids, and nonce to each other
    let nonce = signer.gen_nonce(rng);
    let party_id = signer.party_id;
    let key_ids = signer.key_ids.clone();
    let message = FrostMessage::Signing {
        party_id,
        key_ids: key_ids.clone(),
        nonce: nonce.clone(),
    };

    // Send the message
    tx_to_network.send(message).await.map_err(|err| JobError {
        reason: format!("Error sending FROST message: {err:?}"),
    })?;

    let mut party_key_ids = HashMap::new();
    let mut party_nonces = HashMap::new();

    party_key_ids.insert(party_id, key_ids);
    party_nonces.insert(party_id, nonce);

    while party_nonces.len() < threshold as usize {
        match rx_from_network.next().await {
            Some(FrostMessage::Signing {
                party_id: party_id_recv,
                key_ids,
                nonce,
            }) => {
                if party_id != party_id_recv {
                    party_key_ids.insert(party_id_recv, key_ids);
                    party_nonces.insert(party_id_recv, nonce);
                }
            }

            Some(evt) => {
                logger.warn(format!("Received unexpected FROST signing event: {evt:?}"));
            }

            None => {
                return Err(JobError {
                    reason: "NetListen connection died".to_string(),
                })
            }
        }
    }

    // Sort the vecs
    let party_ids = party_key_ids
        .keys()
        .copied()
        .sorted_by(|a, b| a.cmp(b))
        .collect_vec();
    let party_key_ids = party_key_ids
        .into_iter()
        .sorted_by(|a, b| a.0.cmp(&b.0))
        .flat_map(|r| r.1)
        .collect_vec();
    let party_nonces = party_nonces
        .into_iter()
        .sorted_by(|a, b| a.0.cmp(&b.0))
        .map(|r| r.1)
        .collect_vec();

    // Generate our signature share
    let signature_share = signer.sign(msg, &party_ids, &party_key_ids, &party_nonces);
    let message = FrostMessage::SigningFinal {
        party_id,
        signature_share: signature_share.clone(),
    };
    // Broadcast our signature share to each other
    tx_to_network_final
        .send(message)
        .await
        .map_err(|err| JobError {
            reason: format!("Error sending FROST message: {err:?}"),
        })?;

    let mut signature_shares = HashMap::new();
    signature_shares.insert(party_id, signature_share.clone());

    // Receive n_signers number of shares
    while signature_shares.len() < threshold as usize {
        match rx_from_network_final.next().await {
            Some(FrostMessage::SigningFinal {
                party_id: party_id_recv,
                signature_share,
            }) => {
                if party_id != party_id_recv {
                    signature_shares.insert(party_id_recv, signature_share);
                }
            }

            Some(evt) => {
                logger.warn(format!(
                    "Received unexpected FROST signing final event: {evt:?}"
                ));
            }

            None => {
                return Err(JobError {
                    reason: "NetListen connection died".to_string(),
                })
            }
        }
    }

    // Sort the signature shares
    let signature_shares = signature_shares
        .into_iter()
        .sorted_by(|a, b| a.0.cmp(&b.0))
        .map(|r| r.1)
        .collect_vec();

    let mut sig_agg = wsts::v2::Aggregator::new(num_keys, threshold);
    sig_agg.init(&public_key).map_err(|err| JobError {
        reason: err.to_string(),
    })?;

    sig_agg
        .sign(msg, &party_nonces, &signature_shares, &party_key_ids)
        .map_err(|err| JobError {
            reason: err.to_string(),
        })
}
