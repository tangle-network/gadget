use crate::environments::GadgetEnvironment;
use crate::gadget::message::TangleProtocolMessage;
use crate::gadget::work_manager::TangleWorkManager;
use crate::gadget::Job;
use async_trait::async_trait;
use gadget_core::job::{BuiltExecutableJobWrapper, JobError};
use gadget_core::job_manager::{ProtocolRemote, ShutdownReason, WorkManagerInterface};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

pub struct AsyncProtocolRemote<Env: GadgetEnvironment> {
    pub start_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    pub shutdown_tx: Mutex<Option<tokio::sync::oneshot::Sender<ShutdownReason>>>,
    pub associated_session_id: <Env::JobManager as WorkManagerInterface>::SessionID,
    pub associated_block_id: <Env::JobManager as WorkManagerInterface>::Clock,
    pub associated_retry_id: <Env::JobManager as WorkManagerInterface>::RetryID,
    pub associated_task_id: <Env::JobManager as WorkManagerInterface>::TaskID,
    pub to_async_protocol: tokio::sync::mpsc::UnboundedSender<Env::ProtocolMessage>,
    pub is_done: Arc<AtomicBool>,
}

#[async_trait]
pub trait AsyncProtocol<Env: GadgetEnvironment> {
    type AdditionalParams: Send + Sync + 'static;
    async fn generate_protocol_from(
        &self,
        associated_block_id: <Env::JobManager as WorkManagerInterface>::Clock,
        associated_retry_id: <Env::JobManager as WorkManagerInterface>::RetryID,
        associated_session_id: <Env::JobManager as WorkManagerInterface>::SessionID,
        associated_task_id: <Env::JobManager as WorkManagerInterface>::TaskID,
        protocol_message_rx: UnboundedReceiver<Env::ProtocolMessage>,
        additional_params: Self::AdditionalParams,
    ) -> Result<BuiltExecutableJobWrapper, JobError>;

    async fn create(
        &self,
        session_id: <Env::JobManager as WorkManagerInterface>::SessionID,
        now: <Env::JobManager as WorkManagerInterface>::Clock,
        retry_id: <Env::JobManager as WorkManagerInterface>::RetryID,
        task_id: <Env::JobManager as WorkManagerInterface>::TaskID,
        additional_params: Self::AdditionalParams,
    ) -> Result<Job<Env>, JobError> {
        let is_done = Arc::new(AtomicBool::new(false));
        let (to_async_protocol, protocol_message_rx) = tokio::sync::mpsc::unbounded_channel();
        let (start_tx, start_rx) = tokio::sync::oneshot::channel();
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let async_protocol = self
            .generate_protocol_from(
                now,
                retry_id,
                session_id,
                task_id,
                protocol_message_rx,
                additional_params,
            )
            .await?;

        let remote = AsyncProtocolRemote {
            start_tx: Mutex::new(Some(start_tx)),
            shutdown_tx: Mutex::new(Some(shutdown_tx)),
            associated_block_id: now,
            associated_retry_id: retry_id,
            associated_task_id: task_id,
            associated_session_id: session_id,
            to_async_protocol,
            is_done: is_done.clone(),
        };

        let job_manager_compatible_protocol = crate::helpers::create_job_manager_compatible_job(
            is_done,
            start_rx,
            shutdown_rx,
            async_protocol,
        );

        Ok((remote, job_manager_compatible_protocol))
    }
}

impl<Env: GadgetEnvironment> ProtocolRemote<TangleWorkManager> for AsyncProtocolRemote<Env> {
    fn start(&self) -> Result<(), Env::Error> {
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

    fn session_id(&self) -> <Env::JobManager as WorkManagerInterface>::SessionID {
        self.associated_session_id
    }

    fn started_at(&self) -> <Env::JobManager as WorkManagerInterface>::Clock {
        self.associated_block_id
    }

    fn shutdown(&self, reason: ShutdownReason) -> Result<(), Env::Error> {
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

    fn deliver_message(&self, message: Env::ProtocolMessage) -> Result<(), Env::Error> {
        self.to_async_protocol
            .send(message)
            .map_err(|err| crate::Error::ProtocolRemoteError {
                err: err.to_string(),
            })
    }

    fn has_started(&self) -> bool {
        self.start_tx.lock().is_none()
    }

    fn retry_id(&self) -> <Env::JobManager as WorkManagerInterface>::RetryID {
        self.associated_retry_id
    }
}
