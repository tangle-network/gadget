use crate::error::Error;
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

#[cfg(target_family = "windows")]
pub static OS_COMMAND: &str = "cmd";
#[cfg(target_family = "windows")]
pub static OS_ARG: &str = "/C";

#[cfg(target_family = "unix")]
pub static OS_COMMAND: &str = "sh";
#[cfg(target_family = "unix")]
pub static OS_ARG: &str = "-c";

pub struct ChildInfo {
    pub pid: u32,
    pub tx: broadcast::Sender<String>,
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

pub async fn create_stream(command: &str) -> Result<ChildInfo, std::io::Error> {
    let (tx, _) = broadcast::channel(100);
    let tx_clone = tx.clone();

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let pid = child.id().unwrap();

    Ok(ChildInfo { pid, tx: tx_clone })
}

#[macro_export]
macro_rules! run_command {
    ($command:expr) => {
        create_stream($command)
    };
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
