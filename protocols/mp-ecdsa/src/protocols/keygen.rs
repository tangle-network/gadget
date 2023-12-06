use crate::client::{AccountId, ClientWithApi, MpEcdsaClient};
use crate::keystore::KeystoreBackend;
use crate::network::GossipNetwork;
use crate::protocols::state_machine::{CurrentRoundBlame, StateMachineWrapper};
use crate::protocols::util;
use crate::util::DebugLogger;
use crate::MpEcdsaProtocolConfig;
use async_trait::async_trait;
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use multi_party_ecdsa::gg_2020::state_machine::keygen::Keygen;
use round_based::async_runtime::watcher::StderrWatcher;
use sc_client_api::Backend;
use sp_application_crypto::sp_core::keccak_256;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tangle_primitives::jobs::{JobId, JobType};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::RwLock;
use webb_gadget::gadget::message::GadgetProtocolMessage;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::gadget::{Job, WebbGadgetProtocol, WorkManagerConfig};
use webb_gadget::protocol::AsyncProtocol;
use webb_gadget::{Block, BlockImportNotification, FinalityNotification};

pub struct MpEcdsaKeygenProtocol<B, BE, KBE, C> {
    client: MpEcdsaClient<B, BE, KBE, C>,
    network: GossipNetwork,
    round_blames: Arc<
        RwLock<
            HashMap<
                <WebbWorkManager as WorkManagerInterface>::TaskID,
                tokio::sync::watch::Receiver<CurrentRoundBlame>,
            >,
        >,
    >,
    logger: DebugLogger,
    account_id: AccountId,
}

pub async fn create_protocol(
    _config: &MpEcdsaProtocolConfig,
) -> Result<MpEcdsaKeygenProtocol, Box<dyn Error>> {
    Ok(MpEcdsaKeygenProtocol {})
}

#[async_trait]
impl<B: Block, BE: Backend<B>, C: ClientWithApi<B, BE>, KBE: KeystoreBackend> WebbGadgetProtocol<B>
    for MpEcdsaKeygenProtocol<B, BE, KBE, C>
{
    async fn get_next_jobs(
        &self,
        _notification: &FinalityNotification<B>,
        now: u64,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Vec<Job>>, webb_gadget::Error> {
        let jobs = self
            .client
            .query_jobs_by_validator(self.account_id)
            .await?
            .into_iter()
            .filter(|r| matches!(r.job_type, JobType::DKG(..)));

        let mut ret = vec![];

        for job in jobs {
            let participants = job
                .job_type
                .get_participants()
                .expect("Should exist for DKG");
            if participants.contains(&self.account_id) {
                let task_id = job.job_id.to_be_bytes();
                let task_id = keccak_256(&task_id);
                let session_id = 0; // We are not interested in sessions for the ECDSA protocol
                let retry_id = 0; // TODO: query the job manager for the retry id

                let additional_params = MpEcdsaKeygenExtraParams {
                    i: participants
                        .iter()
                        .position(|p| p == &self.account_id)
                        .expect("Should exist") as u16,
                    t: job.threshold.expect("T should exist for DKG") as u16,
                    n: participants.len() as u16,
                    job_id: job.job_id,
                };

                let job = self
                    .create(session_id, now, retry_id, task_id, additional_params)
                    .await?;

                ret.push(job);
            }
        }

        Ok(Some(ret))
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), webb_gadget::Error> {
        Ok(())
    }

    async fn process_error(
        &self,
        error: webb_gadget::Error,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) {
        log::error!(target: "mp-ecdsa", "Error: {error:?}");
    }

    fn get_work_manager_config(&self) -> WorkManagerConfig {
        WorkManagerConfig {
            interval: None, // Manual polling
            max_active_tasks: 1,
            max_pending_tasks: 1,
        }
    }
}

struct MpEcdsaKeygenExtraParams {
    i: u16,
    t: u16,
    n: u16,
    job_id: JobId,
}

#[async_trait]
impl<B, BE, KBE: KeystoreBackend, C> AsyncProtocol for MpEcdsaKeygenProtocol<B, BE, KBE, C> {
    type AdditionalParams = MpEcdsaKeygenExtraParams;
    async fn generate_protocol_from(
        &self,
        associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let key_store = self.client.key_store.clone();
        let blame = self.round_blames.clone();

        Ok(JobBuilder::new()
            .protocol(async move {
                let (i, t, n) = (
                    additional_params.i,
                    additional_params.t,
                    additional_params.n,
                );
                let keygen = Keygen::new(i, t, n)?;
                let (current_round_blame_tx, current_round_blame_rx) =
                    tokio::sync::watch::channel(CurrentRoundBlame::default());

                self.round_blames
                    .write()
                    .await
                    .insert(associated_task_id, current_round_blame_rx);

                let (tx_to_outbound, rx_async_proto) =
                    util::create_job_manager_to_async_protocol_channel::<Keygen, _>(
                        protocol_message_rx,
                        associated_block_id,
                        associated_retry_id,
                        associated_session_id,
                        associated_task_id,
                        self.network.clone(),
                    );

                let state_machine_wrapper =
                    StateMachineWrapper::new(keygen, current_round_blame_tx, self.logger.clone());
                let local_key = round_based::AsyncProtocol::new(
                    state_machine_wrapper,
                    rx_async_proto,
                    tx_to_outbound,
                )
                .set_watcher(StderrWatcher)
                .run()
                .await?;

                key_store.set(additional_params.job_id, local_key).await?;
                Ok(())
            })
            .post(async move {
                // Check to see if there is any blame at the end of the protocol
                if let Some(blame) = blame.write().await.remove(&associated_task_id) {
                    let blame = blame.borrow();
                    if !blame.blamed_parties.is_empty() {
                        log::error!(target: "mp-ecdsa", "Blame: {blame:?}");
                        return Err(JobError {
                            reason: format!("Blame: {blame:?}"),
                        });
                    }
                }

                Ok(())
            })
            .build())
    }
}
