use crate::error::TestError;
use crate::message::TestProtocolMessage;
use crate::work_manager::{
    AsyncProtocolGenerator, TestAsyncProtocolParameters, TestProtocolRemote, TestWorkManager,
};
use async_trait::async_trait;
use gadget_core::gadget::manager::AbstractGadget;
use gadget_core::job_manager::{ProtocolWorkManager, SendFuture, WorkManagerInterface};
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

/// An AbstractGadget endowed with a WorkerManager, a fake blockchain that delivers FinalityNotifications to the gadgets, and a TestProtocolMessage stream
pub struct TestGadget {
    job_manager: ProtocolWorkManager<TestWorkManager>,
    blockchain_connection: Mutex<tokio::sync::broadcast::Receiver<TestFinalityNotification>>,
    network_connection: Mutex<tokio::sync::mpsc::Receiver<TestProtocolMessage>>,
    // Specifies at which blocks we should start a job on
    run_test_at: Arc<Vec<u64>>,
    async_protocol_generator: Box<dyn AsyncProtocolGenerator>,
}

#[derive(Clone)]
pub struct TestFinalityNotification {
    pub number: u64,
    pub session_id: u64,
}

#[async_trait]
impl AbstractGadget for TestGadget {
    type FinalityNotification = TestFinalityNotification;
    type BlockImportNotification = ();
    type ProtocolMessage = TestProtocolMessage;
    type Error = TestError;

    async fn get_next_finality_notification(&self) -> Option<Self::FinalityNotification> {
        self.blockchain_connection.lock().await.recv().await.ok()
    }

    async fn get_next_block_import_notification(&self) -> Option<Self::BlockImportNotification> {
        // We don't care to test block import notifications in this test gadget
        futures::future::pending().await
    }

    async fn get_next_protocol_message(&self) -> Option<Self::ProtocolMessage> {
        self.network_connection.lock().await.recv().await
    }

    async fn process_finality_notification(
        &self,
        notification: Self::FinalityNotification,
    ) -> Result<(), Self::Error> {
        let now = notification.number;
        let session_id = notification.session_id;

        if self.run_test_at.contains(&now) {
            log::info!("Running test at block {now}");
            let task_hash = now.to_be_bytes();
            let ssid = 0; // Assume SSID = 0 for now
            let (remote, task) =
                create_test_async_protocol(session_id, now, ssid, &*self.async_protocol_generator);
            self.job_manager
                .push_task(task_hash, true, Arc::new(remote), task)
                .map_err(|err| TestError {
                    reason: format!("Failed to push_task: {err:?}"),
                })?;
        }

        Ok(())
    }

    async fn process_block_import_notification(
        &self,
        _notification: Self::BlockImportNotification,
    ) -> Result<(), Self::Error> {
        unreachable!("We don't care to test block import notifications in this test gadget")
    }

    async fn process_protocol_message(
        &self,
        message: Self::ProtocolMessage,
    ) -> Result<(), Self::Error> {
        self.job_manager
            .deliver_message(message)
            .map_err(|err| TestError {
                reason: format!("{err:?}"),
            })
            .map(|_| ())
    }

    async fn process_error(&self, error: Self::Error) {
        log::error!("{error:?}")
    }
}

fn create_test_async_protocol(
    session_id: <TestWorkManager as WorkManagerInterface>::SessionID,
    now: <TestWorkManager as WorkManagerInterface>::Clock,
    ssid: <TestWorkManager as WorkManagerInterface>::SSID,
    proto_gen: &dyn AsyncProtocolGenerator,
) -> (TestProtocolRemote, Pin<Box<dyn SendFuture<'static, ()>>>) {
    let is_done = Arc::new(AtomicBool::new(false));
    let (to_async_protocol, protocol_message_rx) = tokio::sync::mpsc::unbounded_channel();
    let (start_tx, start_rx) = tokio::sync::oneshot::channel();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let params = TestAsyncProtocolParameters {
        is_done: is_done.clone(),
        protocol_message_rx,
        start_rx: Some(start_rx),
        shutdown_rx: Some(shutdown_rx),
    };

    let remote = TestProtocolRemote {
        start_tx: parking_lot::Mutex::new(Some(start_tx)),
        shutdown_tx: parking_lot::Mutex::new(Some(shutdown_tx)),
        associated_block_id: now,
        associated_ssid: ssid,
        associated_session_id: session_id,
        to_async_protocol,
        is_done,
    };

    let future = proto_gen(params);

    (remote, future)
}
