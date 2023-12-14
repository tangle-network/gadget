use crate::client_ext::ClientWithApi;
use crate::network::{RegistantId, ZkNetworkService, ZkSetupPacket};
use crate::protocol::proto_gen::ZkAsyncProtocolParameters;
use async_trait::async_trait;
use gadget_common::gadget::message::GadgetProtocolMessage;
use gadget_common::gadget::work_manager::WebbWorkManager;
use gadget_common::gadget::{Job, WebbGadgetProtocol};
use gadget_common::protocol::AsyncProtocol;
use gadget_common::{BlockImportNotification, Error, FinalityNotification};
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, JobError};
use gadget_core::job_manager::{ProtocolWorkManager, WorkManagerInterface};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedReceiver;

pub mod proto_gen;

pub struct ZkProtocol<B, C, V> {
    pub client: C,
    pub additional_params: V,
    pub network: ZkNetworkService,
    pub _pd: std::marker::PhantomData<B>,
}

pub trait AdditionalProtocolParams: Send + Sync + Clone + 'static {}
impl<T: Send + Sync + Clone + 'static> AdditionalProtocolParams for T {}

#[async_trait]
impl<B: Block, C: ClientWithApi<B>, V: AdditionalProtocolParams> WebbGadgetProtocol<B>
    for ZkProtocol<B, C, V>
where
    <C as ProvideRuntimeApi<B>>::Api:
        pallet_jobs_rpc_runtime_api::JobsApi<B, sp_core::ecdsa::Public>,
{
    async fn get_next_jobs(
        &self,
        _notification: &FinalityNotification<B>,
        now: u64,
        job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<Option<Vec<Job>>, Error> {
        log::info!("Received a finality notification at {now}",);

        if let Some(job) = self.client.get_next_job().await? {
            log::debug!("Found a job {} at {now}", job.job_id);
            let mut task_id = [0u8; 32];
            task_id[..8].copy_from_slice(&job.job_id.to_be_bytes()[..]);

            if job_manager.job_exists(&task_id) {
                return Ok(None);
            }

            let job_specific_params = ZkJobAdditionalParams {
                n_parties: 0, // TODO get from job
                party_id: 0,  // TODO get from job
            };
            let session_id = 0;
            let retry_id = job_manager.latest_retry_id(&task_id).unwrap_or(0);

            let (remote, protocol) = self
                .create(session_id, now, retry_id, task_id, job_specific_params)
                .await
                .map_err(|err| Error::ClientError { err: err.reason })?;

            return Ok(Some(vec![(remote, protocol)]));
        } else {
            log::debug!("No Jobs found at #{now}");
        }

        Ok(None)
    }

    async fn process_block_import_notification(
        &self,
        _notification: BlockImportNotification<B>,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn process_error(
        &self,
        error: Error,
        _job_manager: &ProtocolWorkManager<WebbWorkManager>,
    ) {
        log::error!("Received an error: {error:?}");
    }
}

pub struct ZkJobAdditionalParams {
    n_parties: usize,
    party_id: u32,
}

#[async_trait]
impl<B: Block, C: ClientWithApi<B>, V: AdditionalProtocolParams> AsyncProtocol
    for ZkProtocol<B, C, V>
where
    <C as ProvideRuntimeApi<B>>::Api:
        pallet_jobs_rpc_runtime_api::JobsApi<B, sp_core::ecdsa::Public>,
{
    type AdditionalParams = ZkJobAdditionalParams;

    async fn generate_protocol_from(
        &self,
        associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
        mut protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let mut txs = HashMap::new();
        let mut rxs = HashMap::new();
        for peer_id in 0..additional_params.n_parties {
            // Create 3 multiplexed channels
            let mut txs_for_this_peer = vec![];
            let mut rxs_for_this_peer = vec![];
            for _ in 0..3 {
                let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
                txs_for_this_peer.push(tx);
                rxs_for_this_peer.push(tokio::sync::Mutex::new(rx));
            }

            txs.insert(peer_id as u32, txs_for_this_peer);
            rxs.insert(peer_id as u32, rxs_for_this_peer);
        }

        tokio::task::spawn(async move {
            while let Some(message) = protocol_message_rx.recv().await {
                let message: GadgetProtocolMessage = message;
                match bincode2::deserialize::<proto_gen::MpcNetMessage>(&message.payload) {
                    Ok(deserialized) => {
                        let (source, sid) = (deserialized.source, deserialized.sid);
                        if let Some(txs) = txs.get(&source) {
                            if let Some(tx) = txs.get(sid as usize) {
                                if let Err(err) = tx.send(deserialized) {
                                    log::warn!(
                                    "Failed to forward message from {source} to stream {sid:?} because {err:?}",
                                );
                                }
                            } else {
                                log::warn!(
                                "Failed to forward message from {source} to stream {sid:?} because the tx handle was not found",
                            );
                            }
                        } else {
                            log::warn!(
                            "Failed to forward message from {source} to stream {sid:?} because the tx handle was not found",
                        );
                        }
                    }
                    Err(err) => {
                        log::warn!("Failed to deserialize protocol message: {err:?}");
                    }
                }
            }

            log::warn!("Async protocol message_rx died")
        });

        let other_network_ids = zk_setup_phase(
            additional_params.n_parties,
            &associated_task_id,
            &self.network,
        )
        .await?;

        let params = ZkAsyncProtocolParameters::<_, _, _, B> {
            associated_block_id,
            associated_retry_id,
            associated_session_id,
            associated_task_id,
            rxs,
            party_id: additional_params.party_id,
            n_parties: additional_params.n_parties,
            my_network_id: self.network.my_id(),
            other_network_ids,
            network: self.network.clone(),
            client: self.client.clone(),
            extra_parameters: self.additional_params.clone(),
            _pd: Default::default(),
        };

        Ok(JobBuilder::new()
            .protocol(async move {
                // TODO: build the protocol, using the "params" object as a handle that has the MpcNet implementation
                Ok(())
            })
            .build())
    }
}

/// The goal of the ZK setup phase it to determine the mapping of party_id -> network_id
/// This will allow proper routing of messages to the correct parties.
///
/// This should be run before running any ZK protocols
async fn zk_setup_phase(
    n_parties: usize,
    job_id: &[u8; 32],
    network: &ZkNetworkService,
) -> Result<HashMap<u32, RegistantId>, JobError> {
    if network.is_king() {
        // Wait for n_parties - 1 messages since we are excluding ourselves
        let expected_messages = n_parties - 1;
        let mut ret = HashMap::new();
        loop {
            let packet = network
                .king_only_next_zk_setup_packet(job_id)
                .await
                .ok_or_else(|| JobError {
                    reason: "Failed to receive zk setup packet as king".to_string(),
                })?;

            if let ZkSetupPacket::ClientToKing {
                party_id,
                registry_id,
                ..
            } = packet
            {
                ret.insert(party_id, registry_id);
                if ret.len() == expected_messages {
                    network.king_only_clear_zk_setup_map_for(job_id).await;
                    return Ok(ret);
                }
            } else {
                log::warn!("Received a non-client-to-king packet during zk setup phase");
            }
        }
    } else {
        network
            .client_only_get_zk_setup_result(job_id)
            .await
            .map_err(|err| JobError {
                reason: err.to_string(),
            })
    }
}
