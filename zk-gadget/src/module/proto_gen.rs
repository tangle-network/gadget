use gadget_core::job_manager::{SendFuture, ShutdownReason, WorkManagerInterface};
use parking_lot::Mutex;
use std::fmt::Debug;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use webb_gadget::gadget::work_manager::WebbWorkManager;

pub struct ZkAsyncProtocolParameters<B> {
    pub is_done: Arc<AtomicBool>,
    pub protocol_message_rx: tokio::sync::mpsc::UnboundedReceiver<
        <WebbWorkManager as WorkManagerInterface>::ProtocolMessage,
    >,
    pub associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
    pub associated_ssid: <WebbWorkManager as WorkManagerInterface>::SSID,
    pub associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    pub associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    pub extra_parameters: B,
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

pub trait AsyncProtocolGenerator<B, E>:
    Send + Sync + Fn(ZkAsyncProtocolParameters<B>) -> Pin<Box<dyn SendFuture<'static, Result<(), E>>>>
{
}
impl<
        B: Send + Sync,
        E: Debug,
        T: Send
            + Sync
            + Fn(ZkAsyncProtocolParameters<B>) -> Pin<Box<dyn SendFuture<'static, Result<(), E>>>>,
    > AsyncProtocolGenerator<B, E> for T
{
}

pub fn create_zk_async_protocol<B: Send + Sync + 'static, E: Debug + 'static>(
    session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
    now: <WebbWorkManager as WorkManagerInterface>::Clock,
    ssid: <WebbWorkManager as WorkManagerInterface>::SSID,
    task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    extra_parameters: B,
    proto_gen: &dyn AsyncProtocolGenerator<B, E>,
) -> (ZkProtocolRemote, Pin<Box<dyn SendFuture<'static, ()>>>) {
    let is_done = Arc::new(AtomicBool::new(false));
    let (to_async_protocol, protocol_message_rx) = tokio::sync::mpsc::unbounded_channel();
    let (start_tx, start_rx) = tokio::sync::oneshot::channel();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let proto_hash_hex = hex::encode(task_id);

    let params = ZkAsyncProtocolParameters {
        is_done: is_done.clone(),
        protocol_message_rx,
        associated_block_id: now,
        associated_ssid: ssid,
        associated_session_id: session_id,
        associated_task_id: task_id,
        extra_parameters,
    };

    let remote = ZkProtocolRemote {
        start_tx: Mutex::new(Some(start_tx)),
        shutdown_tx: Mutex::new(Some(shutdown_tx)),
        associated_block_id: now,
        associated_ssid: ssid,
        associated_session_id: session_id,
        to_async_protocol,
        is_done,
    };

    let async_protocol = proto_gen(params);

    let wrapped_future = Box::pin(async move {
        if let Err(err) = start_rx.await {
            log::error!("Failed to start protocol {proto_hash_hex}: {err:?}");
        }

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
    });

    (remote, wrapped_future)
}
