use crate::network::RegistantId;
use async_trait::async_trait;
use bytes::Bytes;
use gadget_core::job_manager::{ProtocolRemote, SendFuture, ShutdownReason, WorkManagerInterface};
use mpc_net::{MpcNet, MpcNetError, MultiplexedStreamID};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use webb_gadget::gadget::message::GadgetProtocolMessage;
use webb_gadget::gadget::network::Network;
use webb_gadget::gadget::work_manager::WebbWorkManager;

pub struct ZkAsyncProtocolParameters<B, N> {
    pub associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
    pub associated_ssid: <WebbWorkManager as WorkManagerInterface>::SSID,
    pub associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    pub associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    rxs: HashMap<u32, Vec<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<MpcNetMessage>>>>,
    pub party_id: RegistantId,
    pub n_parties: usize,
    pub network: N,
    pub extra_parameters: B,
}

#[derive(Serialize, Deserialize)]
struct MpcNetMessage {
    sid: MultiplexedStreamID,
    payload: Bytes,
    source: u32,
}

pub struct ZkProtocolRemote {
    pub start_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    pub shutdown_tx: Mutex<Option<tokio::sync::oneshot::Sender<ShutdownReason>>>,
    pub associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    pub associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
    pub associated_ssid: <WebbWorkManager as WorkManagerInterface>::SSID,
    pub to_async_protocol: tokio::sync::mpsc::UnboundedSender<
        <WebbWorkManager as WorkManagerInterface>::ProtocolMessage,
    >,
    pub is_done: Arc<AtomicBool>,
}

pub trait AsyncProtocolGenerator<B, E, N>:
    Send
    + Sync
    + 'static
    + Fn(ZkAsyncProtocolParameters<B, N>) -> Pin<Box<dyn SendFuture<'static, Result<(), E>>>>
{
}
impl<
        B: Send + Sync,
        E: Debug,
        N: Network,
        T: Send
            + Sync
            + 'static
            + Fn(ZkAsyncProtocolParameters<B, N>) -> Pin<Box<dyn SendFuture<'static, Result<(), E>>>>,
    > AsyncProtocolGenerator<B, E, N> for T
{
}

#[allow(clippy::too_many_arguments)]
pub fn create_zk_async_protocol<B: Send + Sync + 'static, E: Debug + 'static, N: Network>(
    session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    now: <WebbWorkManager as WorkManagerInterface>::Clock,
    ssid: <WebbWorkManager as WorkManagerInterface>::SSID,
    task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    party_id: RegistantId,
    n_parties: usize,
    extra_parameters: B,
    network: N,
    proto_gen: &dyn AsyncProtocolGenerator<B, E, N>,
) -> (ZkProtocolRemote, Pin<Box<dyn SendFuture<'static, ()>>>) {
    let is_done = Arc::new(AtomicBool::new(false));
    let (to_async_protocol, mut protocol_message_rx) = tokio::sync::mpsc::unbounded_channel();
    let (start_tx, start_rx) = tokio::sync::oneshot::channel();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let proto_hash_hex = hex::encode(task_id);

    let mut txs = HashMap::new();
    let mut rxs = HashMap::new();
    for peer_id in 0..n_parties {
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
            let deserialized: MpcNetMessage =
                bincode2::deserialize(&message.payload).expect("Failed to deser message");
            let txs = &txs[&deserialized.source];
            let tx = &txs[deserialized.sid as usize];
            tx.send(deserialized).expect("Failed to send message");
        }
    });

    let params = ZkAsyncProtocolParameters {
        rxs,
        associated_block_id: now,
        party_id,
        n_parties,
        associated_ssid: ssid,
        associated_session_id: session_id,
        associated_task_id: task_id,
        extra_parameters,
        network,
    };

    let remote = ZkProtocolRemote {
        start_tx: Mutex::new(Some(start_tx)),
        shutdown_tx: Mutex::new(Some(shutdown_tx)),
        associated_block_id: now,
        associated_ssid: ssid,
        associated_session_id: session_id,
        to_async_protocol,
        is_done: is_done.clone(),
    };

    let async_protocol = proto_gen(params);

    // This wrapped future enables proper functionality between the async protocol and the
    // job manager
    let wrapped_future = Box::pin(async move {
        if let Err(err) = start_rx.await {
            log::error!("Failed to start protocol {proto_hash_hex}: {err:?}");
        } else {
            tokio::select! {
                res0 = async_protocol => {
                    if let Err(err) = res0 {
                        log::error!("Protocol {proto_hash_hex} failed: {err:?}");
                    } else {
                        log::info!("Protocol {proto_hash_hex} finished");
                    }
                },

                res1 = shutdown_rx => {
                    match res1 {
                        Ok(reason) => {
                            log::info!("Protocol shutdown: {reason:?}");
                        },
                        Err(err) => {
                            log::error!("Protocol shutdown failed: {err:?}");
                        },
                    }
                }
            }
        }

        // Mark the task as done
        is_done.store(true, Ordering::SeqCst);
    });

    (remote, wrapped_future)
}

impl ProtocolRemote<WebbWorkManager> for ZkProtocolRemote {
    fn start(&self) -> Result<(), <WebbWorkManager as WorkManagerInterface>::Error> {
        self.start_tx
            .lock()
            .take()
            .ok_or_else(|| webb_gadget::Error::ProtocolRemoteError {
                err: "Protocol already started".to_string(),
            })?
            .send(())
            .map_err(|_err| webb_gadget::Error::ProtocolRemoteError {
                err: "Unable to start protocol".to_string(),
            })
    }

    fn session_id(&self) -> <WebbWorkManager as WorkManagerInterface>::SessionID {
        self.associated_session_id
    }

    fn set_as_primary(&self) {}

    fn started_at(&self) -> <WebbWorkManager as WorkManagerInterface>::Clock {
        self.associated_block_id
    }

    fn shutdown(
        &self,
        reason: ShutdownReason,
    ) -> Result<(), <WebbWorkManager as WorkManagerInterface>::Error> {
        self.shutdown_tx
            .lock()
            .take()
            .ok_or_else(|| webb_gadget::Error::ProtocolRemoteError {
                err: "Protocol already shutdown".to_string(),
            })?
            .send(reason)
            .map_err(|reason| webb_gadget::Error::ProtocolRemoteError {
                err: format!("Unable to shutdown protocol with status {reason:?}"),
            })
    }

    fn is_done(&self) -> bool {
        self.is_done.load(Ordering::SeqCst)
    }

    fn deliver_message(
        &self,
        message: <WebbWorkManager as WorkManagerInterface>::ProtocolMessage,
    ) -> Result<(), <WebbWorkManager as WorkManagerInterface>::Error> {
        self.to_async_protocol.send(message).map_err(|err| {
            webb_gadget::Error::ProtocolRemoteError {
                err: err.to_string(),
            }
        })
    }

    fn has_started(&self) -> bool {
        self.start_tx.lock().is_none()
    }

    fn ssid(&self) -> <WebbWorkManager as WorkManagerInterface>::SSID {
        self.associated_ssid
    }
}

#[async_trait]
impl<B: Send + Sync, N: Network> MpcNet for ZkAsyncProtocolParameters<B, N> {
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
                associated_ssid: self.associated_ssid,
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
