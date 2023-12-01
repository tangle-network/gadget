use crate::client_ext::ClientWithApi;
use crate::network::RegistantId;
use crate::protocol::AdditionalProtocolParams;
use async_trait::async_trait;
use bytes::Bytes;
use gadget_core::job::{BuiltExecutableJobWrapper, JobError};
use gadget_core::job_manager::WorkManagerInterface;
use mpc_net::{MpcNet, MpcNetError, MultiplexedStreamID};
use serde::{Deserialize, Serialize};
use sp_runtime::traits::Block;
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use tokio::sync::mpsc::UnboundedReceiver;
use webb_gadget::gadget::message::GadgetProtocolMessage;
use webb_gadget::gadget::network::Network;
use webb_gadget::gadget::work_manager::WebbWorkManager;
use webb_gadget::protocol::AsyncProtocol;

pub struct ZkAsyncProtocolParameters<B, N, C, Bl> {
    pub associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
    pub associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
    pub associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    pub associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    rxs: HashMap<u32, Vec<tokio::sync::Mutex<UnboundedReceiver<MpcNetMessage>>>>,
    pub party_id: RegistantId,
    pub n_parties: usize,
    pub network: N,
    pub client: C,
    pub extra_parameters: B,
    _pd: PhantomData<Bl>,
}

#[derive(Serialize, Deserialize)]
struct MpcNetMessage {
    sid: MultiplexedStreamID,
    payload: Bytes,
    source: u32,
}

pub trait AsyncProtocolGenerator<B, E, N, C, Bl>:
    Send + Sync + 'static + Fn(ZkAsyncProtocolParameters<B, N, C, Bl>) -> BuiltExecutableJobWrapper
{
}
impl<
        B: Send + Sync,
        E: Debug,
        N: Network,
        Bl: Block,
        C: ClientWithApi<Bl>,
        T: Send
            + Sync
            + 'static
            + Fn(ZkAsyncProtocolParameters<B, N, C, Bl>) -> BuiltExecutableJobWrapper,
    > AsyncProtocolGenerator<B, E, N, C, Bl> for T
{
}

pub struct ZkProtocolGenerator<B, E, N, C, Bl> {
    pub party_id: RegistantId,
    pub n_parties: usize,
    pub extra_parameters: B,
    pub network: N,
    pub client: C,
    pub proto_gen: Box<dyn AsyncProtocolGenerator<B, E, N, C, Bl>>,
}

#[async_trait]
impl<B: AdditionalProtocolParams, E, N: Network, C: ClientWithApi<Bl>, Bl: Block> AsyncProtocol
    for ZkProtocolGenerator<B, E, N, C, Bl>
{
    type AdditionalParams = ();
    async fn generate_protocol_from(
        &self,
        associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
        associated_retry_id: <WebbWorkManager as WorkManagerInterface>::RetryID,
        associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
        mut protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
        _additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError> {
        let mut txs = HashMap::new();
        let mut rxs = HashMap::new();
        for peer_id in 0..self.n_parties {
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
                match bincode2::deserialize::<MpcNetMessage>(&message.payload) {
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

        let params = ZkAsyncProtocolParameters {
            associated_block_id,
            associated_retry_id,
            associated_session_id,
            associated_task_id,
            rxs,
            party_id: self.party_id,
            n_parties: self.n_parties,
            network: self.network.clone(),
            client: self.client.clone(),
            extra_parameters: self.extra_parameters.clone(),
            _pd: Default::default(),
        };

        Ok((self.proto_gen)(params))
    }
}

#[async_trait]
impl<B: Send + Sync, N: Network, C: ClientWithApi<Bl>, Bl: Block> MpcNet
    for ZkAsyncProtocolParameters<B, N, C, Bl>
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

        self.network
            .send_message(GadgetProtocolMessage {
                associated_block_id: self.associated_block_id,
                associated_session_id: self.associated_session_id,
                associated_retry_id: self.associated_retry_id,
                from: self.party_id,
                to: Some(id),
                task_hash: self.associated_task_id,
                payload: bincode2::serialize(&inner_payload).expect("Failed to serialize message"),
            })
            .await
            .map_err(|err| MpcNetError::Protocol {
                err: err.to_string(),
                party: self.party_id,
            })
    }
}
