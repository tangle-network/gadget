pub use futures::StreamExt;
pub use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
pub use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, ReadBuf};
pub use tokio::process::{Child, Command};
use tokio::sync::{broadcast, oneshot, Mutex};
use sysinfo::{System, Pid};
use std::ffi::OsString;

#[cfg(target_family = "windows")]
pub static OS_COMMAND: &str = "cmd";
#[cfg(target_family = "windows")]
pub static OS_ARG: &str = "/C";

#[cfg(target_family = "unix")]
pub static OS_COMMAND: &str = "sh";
#[cfg(target_family = "unix")]
pub static OS_ARG: &str = "-c";

pub struct ChildInfo {
    pub child: Child,
    pub process_name: OsString,
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
    ($cmd:expr) => {{
        async {
            let mut command = craft_child_process!($cmd);
            let child = command.spawn()?;
            let pid = child.id().ok_or_else(|| Error::ProcessNotFound(Pid::from_u32(0)))?;
            
            let process_name = get_process_info(pid)
                .ok_or(Error::ProcessNotFound(Pid::from_u32(pid)))?;

            let stream = create_stream(child, process_name.clone()).await;
            
            Ok::<_, Error>(GadgetProcess::new(
                $cmd.to_string(),
                pid,
                Vec::new(),
                stream,
                process_name,
            ))
        }.await
    }};
}

pub(crate) async fn create_stream(
    mut child: Child, 
    tx: broadcast::Sender<String>, 
    ready_tx: oneshot::Sender<()>, 
    process_name: OsString
) -> (broadcast::Receiver<String>, oneshot::Receiver<()>) {
    const CHILD_PROCESS_TIMEOUT: Duration = Duration::from_millis(10000);
    const MAX_CONSECUTIVE_NONE: usize = 5;

    let rx = tx.subscribe();
    let first_output_tx = Arc::new(Mutex::new(Some(ready_tx)));
    let first_output_tx_stderr = Arc::clone(&first_output_tx);

    let stdout = child.stdout.take().expect("Failed to take stdout");
    let stderr = child.stderr.take().expect("Failed to take stderr");
    
    let tx_clone = tx.clone();
    let process_name_clone = process_name.clone();
    
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = tx_clone.send(format!("[{}] {}", process_name_clone.to_string_lossy(), line));
            let guard = first_output_tx.lock().await;
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(());
            }
        }
    });

    let tx_clone = tx;
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = tx_clone.send(format!("[{}] {}", process_name.to_string_lossy(), line));
            let guard = first_output_tx_stderr.lock().await;
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(());
            }
        }
    });

    tokio::spawn(async move {
        let status = child.wait().await.expect("Failed to wait for child process");
        gadget_logging::info!("Child process ended with status: {}", status);
    });

    let (ready_tx, ready_rx) = oneshot::channel();
    let guard = first_output_tx.lock().await;
    if let Some(tx) = guard.as_ref() {
        let _ = tx.send(());
    }
    
    (rx, ready_rx)
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
