use crate::gadget::message::GadgetProtocolMessage;
use crate::gadget::work_manager::WebbWorkManager;
use async_trait::async_trait;
use gadget_core::job::{BuiltExecutableJobWrapper, JobError};
use gadget_core::job_manager::{ProtocolRemote, ShutdownReason, WorkManagerInterface};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

pub struct AsyncProtocolRemote {
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

#[async_trait]
pub trait AsyncProtocol {
    async fn generate_protocol_from(
        &self,
        associated_block_id: <WebbWorkManager as WorkManagerInterface>::Clock,
        associated_ssid: <WebbWorkManager as WorkManagerInterface>::SSID,
        associated_session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
        associated_task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<GadgetProtocolMessage>,
    ) -> Result<BuiltExecutableJobWrapper, JobError>;

    async fn create(
        &self,
        session_id: <WebbWorkManager as WorkManagerInterface>::SessionID,
        now: <WebbWorkManager as WorkManagerInterface>::Clock,
        ssid: <WebbWorkManager as WorkManagerInterface>::SSID,
        task_id: <WebbWorkManager as WorkManagerInterface>::TaskID,
    ) -> Result<(AsyncProtocolRemote, BuiltExecutableJobWrapper), JobError> {
        let is_done = Arc::new(AtomicBool::new(false));
        let (to_async_protocol, protocol_message_rx) = tokio::sync::mpsc::unbounded_channel();
        let (start_tx, start_rx) = tokio::sync::oneshot::channel();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let proto_hash_hex = hex::encode(task_id);
        let async_protocol = self
            .generate_protocol_from(now, ssid, session_id, task_id, protocol_message_rx)
            .await?;

        let remote = AsyncProtocolRemote {
            start_tx: Mutex::new(Some(start_tx)),
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
            associated_block_id: now,
            associated_ssid: ssid,
            associated_session_id: session_id,
            to_async_protocol,
            is_done: is_done.clone(),
        };

        let job_manager_compatible_protocol = crate::helpers::create_job_manager_compatible_job(
            proto_hash_hex,
            is_done,
            start_rx,
            shutdown_rx,
            async_protocol,
        );

        Ok((remote, job_manager_compatible_protocol))
    }
}

impl ProtocolRemote<WebbWorkManager> for AsyncProtocolRemote {
    fn start(&self) -> Result<(), <WebbWorkManager as WorkManagerInterface>::Error> {
        self.start_tx
            .lock()
            .take()
            .ok_or_else(|| crate::Error::ProtocolRemoteError {
                err: "Protocol already started".to_string(),
            })?
            .send(())
            .map_err(|_err| crate::Error::ProtocolRemoteError {
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
            .ok_or_else(|| crate::Error::ProtocolRemoteError {
                err: "Protocol already shutdown".to_string(),
            })?
            .send(reason)
            .map_err(|reason| crate::Error::ProtocolRemoteError {
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
        self.to_async_protocol
            .send(message)
            .map_err(|err| crate::Error::ProtocolRemoteError {
                err: err.to_string(),
            })
    }

    fn has_started(&self) -> bool {
        self.start_tx.lock().is_none()
    }

    fn ssid(&self) -> <WebbWorkManager as WorkManagerInterface>::SSID {
        self.associated_ssid
    }
}
