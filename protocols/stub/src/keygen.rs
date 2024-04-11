use std::collections::HashMap;
use std::sync::Arc;
use futures::StreamExt;
use itertools::Itertools;
use sp_core::{ByteArray, ecdsa, keccak_256, Pair, sha2_256};
use tokio::sync::mpsc::UnboundedReceiver;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::prelude::{GadgetProtocolMessage, JobTypeExt, WorkManager};
use gadget_common::{BuiltExecutableJobWrapper, JobBuilder, JobError, ProtocolWorkManager, WorkManagerInterface};
use gadget_common::client::ClientWithApi;
use gadget_common::config::Network;
use gadget_common::gadget::message::UserID;
use gadget_common::keystore::KeystoreBackend;
use serde::{Deserialize, Serialize};
use gadget_common::channels::{MaybeReceiver, MaybeSender, MaybeSenderReceiver};
use rand::random;
use tangle_primitives::jobs::JobId;
use gadget_common::tangle_runtime::api::jobs::events::job_submitted::RoleType;
use gadget_common::tangle_runtime::jobs;
use gadget_common::tangle_runtime::roles::tss::ThresholdSignatureRoleType;
use gadget_common::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use gadget_common::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::jobs::tss::DigitalSignatureScheme;
use crate::StubKeygenProtocol;

#[derive(Clone)]
pub struct XorStubKeygenProtocolParameters {
    #[allow(dead_code)]
    n: usize,
    t: usize,
    user_id_mapping: Arc<HashMap<UserID, ecdsa::Public>>,
    job_id: JobId,
}

pub(crate) async fn create_next_job(
    job: JobInitMetadata,
    _wm: &ProtocolWorkManager<WorkManager>,
) -> Result<XorStubKeygenProtocolParameters, gadget_common::Error> {
    let n = job.participants_role_ids.len();
    let t = job
        .job_type
        .get_threshold()
        .expect("Should exist for phase1 job") as _;
    let job_id = job.job_id;
    let user_id_mapping = Arc::new(
        job.participants_role_ids
            .into_iter()
            .enumerate()
            .map(|(id, p)| (id as _, p))
            .collect(),
    );
    Ok(XorStubKeygenProtocolParameters {
        n,
        t,
        user_id_mapping,
        job_id,
    })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeygenProtocolPacket {
    sender: UserID,
    receiver: Option<UserID>,
    key_share: u64,
}

impl MaybeSenderReceiver for KeygenProtocolPacket {
    fn maybe_sender(&self) -> MaybeSender {
        MaybeSender::Myself
    }

    fn maybe_receiver(&self) -> MaybeReceiver {
        if let Some(reciever) = self.receiver {
            MaybeReceiver::P2P(reciever)
        } else {
            MaybeReceiver::Broadcast
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeygenProtocolPacketGossip {
    sender: UserID,
    receiver: Option<UserID>,
    signature: Vec<u8>,
}

impl MaybeSenderReceiver for crate::keygen::KeygenProtocolPacketGossip {
    fn maybe_sender(&self) -> MaybeSender {
        MaybeSender::Myself
    }

    fn maybe_receiver(&self) -> MaybeReceiver {
        if let Some(reciever) = self.receiver {
            MaybeReceiver::P2P(reciever)
        } else {
            MaybeReceiver::Broadcast
        }
    }
}

pub(crate) async fn generate_protocol_from<
    C: ClientWithApi + 'static,
    N: Network,
    KBE: KeystoreBackend,
>(
    config: &StubKeygenProtocol<C, N, KBE>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
    protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
    params: XorStubKeygenProtocolParameters,
) -> Result<BuiltExecutableJobWrapper, JobError> {
    let my_id = config.key_store.pair().public();
    let user_id_mapping = params.user_id_mapping.clone();
    let participants = params
        .user_id_mapping
        .iter()
        .sorted_by_key(|r| r.0)
        .map(|(_, p)| *p)
        .collect::<Vec<_>>();
    let t = params.t;
    let job_id = params.job_id;
    let client = config.pallet_tx.clone();
    let key_store = config.key_store.clone();
    let my_idx = user_id_mapping
        .iter()
        .find(|(_, p)| *p == &my_id)
        .map(|(idx, _)| *idx)
        .unwrap();
    let network = config.clone();
    let logger = config.logger.clone();
    let logger_clone = logger.clone();
    logger.info("Starting AsyncProtocol - Keygen");
    let key_store_clone = config.key_store.clone();

    let (tx, mut rx, tx_pk_gossip, mut rx_pk_gossip) =
        gadget_common::channels::create_job_manager_to_async_protocol_channel_split::<
            _,
            KeygenProtocolPacket,
            KeygenProtocolPacketGossip,
        >(
            protocol_message_rx,
            associated_block_id,
            associated_retry_id,
            associated_session_id,
            associated_task_id,
            user_id_mapping,
            my_id,
            network,
            logger,
        );

    let result = Arc::new(tokio::sync::Mutex::new(None));
    let result_clone = result.clone();

    Ok(JobBuilder::new()
        .protocol(async move {
            // Step 1: Generate a random u64
            let key_share = random::<u64>();
            let packet = KeygenProtocolPacket {
                sender: my_idx,
                receiver: None,
                key_share,
            };

            let mut key_shares = vec![key_share];

            tx.unbounded_send(packet).unwrap();

            // Collect t + 1 key shares
            while key_shares.len() < (t + 1) {
                let packet = rx.next().await.unwrap().unwrap();
                key_shares.push(packet.key_share);
            }

            let final_key: u64 = key_shares.into_iter().fold(0, |r0, r1| r0.wrapping_add(r1));

            // Sign and broadcast, collect t + 1 signatures
            let mut signatures = HashMap::new();
            //let signature = xor_sign(final_key, &final_key.to_be_bytes());
            let key_hashed = keccak_256(&final_key.to_be_bytes());
            let signature = key_store.pair().sign_prehashed(&key_hashed).0.to_vec();
            signatures.insert(my_idx, signature.clone());

            tx_pk_gossip
                .send(KeygenProtocolPacketGossip {
                    sender: my_idx,
                    receiver: None,
                    signature,
                })
                .unwrap();

            while signatures.len() < (t + 1) {
                let packet = rx_pk_gossip.recv().await.unwrap();
                signatures.insert(packet.sender, packet.signature);
            }

            let signatures = signatures
                .into_iter()
                .sorted_by_key(|r| r.0)
                .map(|(_, s)| s)
                .collect::<Vec<_>>();

            result.lock().await.replace((final_key, signatures));
            Ok(())
        })
        .post(async move {
            if let Some((key, signatures)) = result_clone.lock().await.take() {
                let job_hash = sha2_256(&job_id.to_be_bytes());
                key_store_clone.set(&job_hash, key).await.unwrap();

                let result = jobs::JobResult::DKGPhaseOne(jobs::tss::DKGTSSKeySubmissionResult {
                    signature_scheme: DigitalSignatureScheme::EcdsaSecp256k1,
                    key: BoundedVec(key.to_be_bytes().to_vec()),
                    participants: BoundedVec(
                        participants
                            .into_iter()
                            .map(|p| BoundedVec(p.to_raw_vec()))
                            .collect::<Vec<_>>(),
                    ),
                    signatures: BoundedVec(
                        signatures.into_iter().map(BoundedVec).collect::<Vec<_>>(),
                    ),
                    threshold: t as u8,
                    chain_code: None,
                    __ignore: Default::default(),
                });

                client
                    .submit_job_result(
                        RoleType::Tss(ThresholdSignatureRoleType::XorStub),
                        job_id,
                        result,
                    )
                    .await
                    .map_err(|e| JobError {
                        reason: e.to_string(),
                    })?;
            }

            logger_clone.info("Finished AsyncProtocol - Keygen");

            Ok(())
        })
        .build())
}

pub fn xor_sign(key: u64, bytes: &[u8]) -> Vec<u8> {
    let be_bytes = key.to_be_bytes();
    let mut idx = 0;
    let mut ret = vec![];
    for value in bytes {
        ret.push(*value ^ be_bytes[idx]);
        idx += 1;
        idx %= 8;
    }

    ret
}
