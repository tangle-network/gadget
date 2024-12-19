pub use futures::StreamExt;
use std::ffi::OsString;
pub use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Pid, System};
pub use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, ReadBuf};
pub use tokio::process::{Child, Command};
use tokio::sync::{broadcast, oneshot, Mutex};
use crate::error::Error;

#[cfg(target_family = "windows")]
pub static OS_COMMAND: &str = "cmd";
#[cfg(target_family = "windows")]
pub static OS_ARG: &str = "/C";

#[cfg(target_family = "unix")]
pub static OS_COMMAND: &str = "sh";
#[cfg(target_family = "unix")]
pub static OS_ARG: &str = "-c";

pub struct ChildInfo {
    pub tx: broadcast::Receiver<String>,
    pub ready_rx: oneshot::Receiver<()>,
    pub pid: u32,
}

pub fn get_process_info(pid: u32) -> Option<OsString> {
    let mut attempts = 5;
    while attempts > 0 {
        let mut s = System::new_all();
        s.refresh_all();

        if let Some(process) = s.process(Pid::from_u32(pid)) {
            return Some(process.name().to_os_string());
        }

        attempts -= 1;
        if attempts > 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    None
}

#[macro_export]
macro_rules! craft_child_process {
    ($cmd:expr) => {{
        let mut command = Command::new(OS_COMMAND);
        command.args(&[OS_ARG, $cmd]);
        command.stderr(Stdio::piped())
              .stdout(Stdio::piped());
        command
    }};
    ($cmd:expr, $($args:expr),*) => {{
        let mut command = Command::new(OS_COMMAND);
        command.args(&[OS_ARG, $cmd]);
        $(
            command.arg($args);
        )*
        command.stdout(Stdio::piped())
               .stderr(Stdio::piped());
        command
    }};
}

#[macro_export]
macro_rules! run_command {
    ($command:expr) => {{
        async {
            let mut command = Command::new(OS_COMMAND);
            command.arg(OS_ARG).arg($command);
            command.stdout(Stdio::piped());
            
            let child = command.spawn()?;
            let process_name = $command.to_string();
            let (ready_tx, ready_rx) = oneshot::channel();
            let (tx, _) = broadcast::channel(100);
            
            tokio::spawn(create_stream(child, tx.clone(), ready_tx, process_name));
            
            ready_rx.await?;
            
            Ok(ChildInfo {
                tx: tx.subscribe(),
                ready_rx: ready_rx,
                pid: child.id().unwrap_or(0),
            })
        }
    }};
}

pub(crate) async fn create_stream(
    mut child: Child,
    tx: broadcast::Sender<String>,
    ready_tx: oneshot::Sender<()>,
    process_name: String,
) -> Result<(), Error> {
    let stdout = child.stdout.take().ok_or_else(|| {
        Error::StreamError(
            Pid::from_u32(0),
            "Failed to capture stdout".to_string(),
        )
    })?;

    let first_output_tx = Arc::new(Mutex::new(Some(ready_tx)));
    let first_output_tx_clone = Arc::clone(&first_output_tx);

    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Err(e) = tx.send(line) {
                log::error!("Failed to send output: {}", e);
                break;
            }

            // Signal that we've received the first output
            if let Ok(mut guard) = first_output_tx.lock().await {
                if let Some(tx) = guard.take() {
                    let _ = tx.send(());
                }
            }
        }
    });

    // Wait for first output or process exit
    tokio::select! {
        _ = first_output_tx_clone.lock() => Ok(()),
        status = child.wait() => {
            if !status?.success() {
                Err(Error::UnexpectedExit(status?.code()))
            } else {
                Ok(())
            }
        }
    }
}

#[allow(dead_code)]
fn handle_output(
    result: std::io::Result<()>,
    read_buf: &ReadBuf<'_>,
    tx: &broadcast::Sender<String>,
    source: &str,
) {
    match result {
        Ok(()) => {
            let filled = read_buf.filled();
            if filled.is_empty() {
                return;
            }
            if let Ok(line) = std::str::from_utf8(filled) {
                let message = format!("[{}] {}", source, line.trim());
                if tx.send(message.clone()).is_err() {
                    // there are no active receivers, stop the task
                    gadget_logging::error!("Error sending message: {}", message);
                }
            }
        }
        Err(e) => {
            gadget_logging::error!("Error reading from {}: {}", source, e);
        }
    }
}
