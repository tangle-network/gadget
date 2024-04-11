use std::collections::HashMap;
use std::sync::Arc;
use futures::StreamExt;
use itertools::Itertools;
use sp_core::{ecdsa, Pair, sha2_256};
use tokio::sync::mpsc::UnboundedReceiver;
use gadget_common::gadget::JobInitMetadata;
use gadget_common::prelude::{GadgetProtocolMessage, WorkManager};
use gadget_common::{BuiltExecutableJobWrapper, JobBuilder, JobError, ProtocolWorkManager, WorkManagerInterface};
use gadget_common::client::{ClientWithApi, JobTypeExt};
use gadget_common::config::Network;
use gadget_common::gadget::message::UserID;
use gadget_common::keystore::KeystoreBackend;
use serde::{Deserialize, Serialize};
use gadget_common::channels::{MaybeReceiver, MaybeSender, MaybeSenderReceiver};
use tangle_primitives::jobs::JobId;
use gadget_common::tangle_runtime::{DigitalSignatureScheme, jobs};
use gadget_common::tangle_runtime::api::jobs::events::job_submitted::RoleType;
use gadget_common::tangle_runtime::jobs::JobType;
use gadget_common::tangle_runtime::roles::tss::ThresholdSignatureRoleType;
use gadget_common::tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
use crate::keygen::xor_sign;
use crate::StubSigningProtocol;

#[derive(Clone)]
pub struct XorStubSigningProtocolParameters {
    #[allow(dead_code)]
    n: usize,
    t: usize,
    user_id_mapping: Arc<HashMap<UserID, ecdsa::Public>>,
    job_id: JobId,
    phase1_job_id: JobId,
    msg: Vec<u8>,
}

pub(crate) async fn create_next_job(
    job: JobInitMetadata,
    _wm: &ProtocolWorkManager<WorkManager>,
) -> Result<XorStubSigningProtocolParameters, gadget_common::Error> {
    let n = job.participants_role_ids.len();
    let phase1_job = job.phase1_job.expect("Should exist for a phase 2 job");
    let t = phase1_job.get_threshold().expect("Threshold should exist") as _;
    let phase1_job_id = job
        .job_type
        .get_phase_one_id()
        .expect("Shopuld exist for a phase 2 job");
    let job_id = job.job_id;
    let user_id_mapping = Arc::new(
        job.participants_role_ids
            .into_iter()
            .enumerate()
            .map(|(id, p)| (id as _, p))
            .collect(),
    );
    let JobType::DKGTSSPhaseTwo(p2_job) = job.job_type else {
        panic!("Expected DKGTSSPhaseTwo job type")
    };
    let msg = p2_job.submission.0;
    Ok(XorStubSigningProtocolParameters {
        n,
        t,
        user_id_mapping,
        job_id,
        msg,
        phase1_job_id,
    })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SigningProtocolPacket {
    sender: UserID,
    receiver: Option<UserID>,
    signature: Vec<u8>,
}

impl MaybeSenderReceiver for SigningProtocolPacket {
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
    config: &StubSigningProtocol<C, N, KBE>,
    associated_block_id: <WorkManager as WorkManagerInterface>::Clock,
    associated_retry_id: <WorkManager as WorkManagerInterface>::RetryID,
    associated_session_id: <WorkManager as WorkManagerInterface>::SessionID,
    associated_task_id: <WorkManager as WorkManagerInterface>::TaskID,
    protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
    params: XorStubSigningProtocolParameters,
) -> Result<BuiltExecutableJobWrapper, JobError> {
    let my_id = config.key_store.pair().public();
    let user_id_mapping = params.user_id_mapping.clone();
    let t = params.t;
    let job_id = params.job_id;
    let msg = params.msg;
    let phase1_job_id = params.phase1_job_id;
    let client = config.pallet_tx.clone();
    let msg_clone = msg.clone();
    let key_store = config.key_store.clone();

    let my_idx = user_id_mapping
        .iter()
        .find(|(_, p)| *p == &my_id)
        .map(|(idx, _)| *idx)
        .unwrap();
    let (tx, mut rx, _, _) =
        gadget_common::channels::create_job_manager_to_async_protocol_channel_split::<
            _,
            SigningProtocolPacket,
            (),
        >(
            protocol_message_rx,
            associated_block_id,
            associated_retry_id,
            associated_session_id,
            associated_task_id,
            user_id_mapping,
            my_id,
            config.clone(),
            config.logger.clone(),
        );

    let result = Arc::new(tokio::sync::Mutex::new(None));
    let result_clone = result.clone();

    Ok(JobBuilder::new()
        .protocol(async move {
            // Step 1: Get the local key
            let job_id_hash = sha2_256(&phase1_job_id.to_be_bytes());
            let signing_key = key_store.get::<u64>(&job_id_hash).await.unwrap().unwrap();

            // Step 2: Sign the message with the local key
            let signed_message = xor_sign(signing_key, &msg);

            let packet = SigningProtocolPacket {
                sender: my_idx,
                receiver: None,
                signature: signed_message.clone(),
            };

            tx.unbounded_send(packet).unwrap();

            let mut received_signatures = HashMap::new();
            received_signatures.insert(my_idx, signed_message.clone());

            // Step 3: Collect t + 1 signatures and ensure all are equal to local
            while received_signatures.len() < (t + 1) {
                let packet = rx.next().await.unwrap().unwrap();
                if packet.signature != signed_message {
                    return Err(JobError {
                        reason: "Invalid party signature".to_string(),
                    });
                }
                received_signatures.insert(packet.sender, packet.signature);
            }

            let signatures = received_signatures
                .into_iter()
                .sorted_by_key(|r| r.0)
                .map(|(_, s)| s)
                .collect::<Vec<_>>();

            result.lock().await.replace(signatures);
            Ok(())
        })
        .post(async move {
            if let Some(mut signatures) = result_clone.lock().await.take() {
                let result = jobs::JobResult::DKGPhaseTwo(jobs::tss::DKGTSSSignatureResult {
                    signature_scheme: DigitalSignatureScheme::XorStub,
                    data: BoundedVec(msg_clone),
                    signature: BoundedVec(signatures.pop().unwrap()),
                    verifying_key: BoundedVec(vec![]), // This value is ignored for signing, as it will be fetched from the phase 1 job result
                    derivation_path: None,
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

            Ok(())
        })
        .build())
}
