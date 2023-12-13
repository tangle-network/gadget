use gadget_core::job::ExecutableJob;
use gadget_core::job::JobError;
use gadget_core::job::{BuiltExecutableJobWrapper, JobBuilder, ProceedWithExecution};
use gadget_core::job_manager::ShutdownReason;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Wraps an async protocol with logic that makes it compatible with the job manager
pub fn create_job_manager_compatible_job<T: Display + Send + Clone + 'static>(
    task_name: T,
    is_done: Arc<AtomicBool>,
    start_rx: tokio::sync::oneshot::Receiver<()>,
    shutdown_rx: tokio::sync::oneshot::Receiver<ShutdownReason>,
    mut async_protocol: BuiltExecutableJobWrapper,
) -> BuiltExecutableJobWrapper {
    let task_name_cloned = task_name.clone();
    let pre_hook = async move {
        match start_rx.await {
            Ok(_) => Ok(ProceedWithExecution::True),
            Err(err) => {
                log::error!("Protocol {task_name_cloned} failed to receive start signal: {err:?}");
                Ok(ProceedWithExecution::False)
            }
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
        tokio::select! {
            res0 = async_protocol.execute() => {
                if let Err(err) = res0 {
                    log::error!("Protocol {task_name} failed: {err:?}");
                    Err(JobError::from(err.to_string()))
                } else {
                    log::info!("Protocol {task_name} finished");
                    Ok(())
                }
            },

            res1 = shutdown_rx => {
                match res1 {
                    Ok(reason) => {
                        log::info!("Protocol {task_name} shutdown: {reason:?}");
                        Ok(())
                    },
                    Err(err) => {
                        log::error!("Protocol {task_name} shutdown failed: {err:?}");
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
