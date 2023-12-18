use crate::client_ext::{AccountId, ClientWithApi};
use crate::network::RegistantId;
use async_trait::async_trait;
use bytes::Bytes;
use gadget_common::gadget::message::GadgetProtocolMessage;
use gadget_common::gadget::network::Network;
use gadget_common::gadget::work_manager::WebbWorkManager;
use gadget_core::job_manager::WorkManagerInterface;
use mpc_net::{MpcNet, MpcNetError, MultiplexedStreamID};
use serde::{Deserialize, Serialize};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block;
use std::collections::HashMap;
use std::marker::PhantomData;
use tokio::sync::mpsc::UnboundedReceiver;

pub struct ZkAsyncProtocolParameters<B, N, C, Bl> {
    pub associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
    pub associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
    pub associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    pub associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    pub(crate) rxs: HashMap<u32, Vec<tokio::sync::Mutex<UnboundedReceiver<MpcNetMessage>>>>,
    pub party_id: u32,
    pub n_parties: usize,
    pub network: N,
    pub client: C,
    pub extra_parameters: B,
    pub my_network_id: RegistantId,
    // Mapping from party_id to network_id, needed for routing messages
    pub other_network_ids: HashMap<u32, RegistantId>,
    pub(crate) _pd: PhantomData<Bl>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct MpcNetMessage {
    pub(crate) sid: MultiplexedStreamID,
    payload: Bytes,
    pub(crate) source: u32,
}

#[async_trait]
impl<B: Send + Sync, N: Network, C: ClientWithApi<Bl>, Bl: Block> MpcNet
    for ZkAsyncProtocolParameters<B, N, C, Bl>
where
    <C as ProvideRuntimeApi<Bl>>::Api: pallet_jobs_rpc_runtime_api::JobsApi<Bl, AccountId>,
{
    fn n_parties(&self) -> usize {
        self.n_parties
    }

    fn party_id(&self) -> u32 {
        self.party_id
    }

    fn is_init(&self) -> bool {
        true
    }

    async fn recv_from(&self, id: u32, sid: MultiplexedStreamID) -> Result<Bytes, MpcNetError> {
        self.rxs
            .get(&id)
            .ok_or_else(|| MpcNetError::Protocol {
                err: format!("There is no rx handle from {id}"),
                party: self.party_id,
            })?
            .get(sid as usize)
            .ok_or_else(|| MpcNetError::Protocol {
                err: format!("There is no rx handle from {id} on stream {sid:?}"),
                party: self.party_id,
            })?
            .lock()
            .await
            .recv()
            .await
            .ok_or_else(|| MpcNetError::Protocol {
                err: "Failed to receive message".to_string(),
                party: self.party_id,
            })
            .map(|msg| msg.payload)
    }

    async fn send_to(
        &self,
        id: u32,
        bytes: Bytes,
        sid: MultiplexedStreamID,
    ) -> Result<(), MpcNetError> {
        let inner_payload = MpcNetMessage {
            sid,
            payload: bytes,
            source: self.party_id,
        };

        let to_network_id =
            self.other_network_ids
                .get(&id)
                .copied()
                .ok_or_else(|| MpcNetError::Protocol {
                    err: format!("There is no network id for party {id}"),
                    party: self.party_id,
                })?;

        self.network
            .send_message(GadgetProtocolMessage {
                associated_block_id: self.associated_block_id,
                associated_session_id: self.associated_session_id,
                associated_retry_id: self.associated_retry_id,
                from: self.party_id,
                to: Some(id),
                task_hash: self.associated_task_id,
                payload: bincode2::serialize(&inner_payload).expect("Failed to serialize message"),
                from_network_id: Some(self.my_network_id),
                to_network_id: Some(to_network_id),
            })
            .await
            .map_err(|err| MpcNetError::Protocol {
                err: err.to_string(),
                party: self.party_id,
            })
    }
}
