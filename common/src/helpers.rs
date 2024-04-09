use gadget_core::job::ExecutableJob;
use gadget_core::job::JobError;
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, ProceedWithExecution};
use gadget_core::job_manager::ShutdownReason;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Wraps an async protocol with logic that makes it compatible with the job manager
pub fn create_job_manager_compatible_job(
    is_done: Arc<AtomicBool>,
    start_rx: gadget_io::tokio::sync::oneshot::Receiver<()>,
    shutdown_rx: gadget_io::tokio::sync::oneshot::Receiver<ShutdownReason>,
    mut async_protocol: BuiltExecutableJobWrapper,
) -> BuiltExecutableJobWrapper {
    let pre_hook = async move {
        match start_rx.await {
            Ok(_) => Ok(ProceedWithExecution::True),
            Err(err) => Err(JobError::from(format!("Failed to start job: {err:?}"))),
        }
    };

    let post_hook = async move {
        // Mark the task as done
        is_done.store(true, Ordering::SeqCst);
        Ok(())
    };

    // This wrapped future enables proper functionality between the async protocol and the
    // job manager
    let wrapped_future = async move {
        gadget_io::tokio::select! {
            res0 = async_protocol.execute() => res0,
            res1 = shutdown_rx => {
                match res1 {
                    Ok(reason) => {
                        match reason {
                            ShutdownReason::DropCode => {
                                Ok(())
                            },
                            ShutdownReason::Stalled => {
                                Err(JobError::from("Stalled".to_string()))
                            },
                        }
                    },
                    Err(err) => {
                        Err(JobError::from(err.to_string()))
                    },
                }
            }
        }
    };

    JobBuilder::default()
        .pre(pre_hook)
        .protocol(wrapped_future)
        .post(post_hook)
        .build()
}
