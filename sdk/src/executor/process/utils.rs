pub use futures::{FutureExt, StreamExt};
pub use std::process::Stdio;
use std::time::Duration;
pub use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, ReadBuf};
pub use tokio::process::{Child, Command};
use tokio::sync::broadcast;
use tokio::time::sleep;

#[cfg(target_family = "unix")]
pub(crate) static OS_COMMAND: (&str, &str) = ("sh", "-c");
#[cfg(target_family = "windows")]
pub(crate) static OS_COMMAND: (&str, &str) = ("cmd", "/C");

#[macro_export]
macro_rules! craft_child_process {
    ($cmd:expr) => {{
        let (cmd, arg) = OS_COMMAND;
        Command::new(cmd).args(vec![arg, $cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect(&format!("Failed to execute: {:?} - Input should be a valid command", $cmd))
    }};
    ($cmd:expr, $($args:expr),*) => {{
        let (cmd, arg) = OS_COMMAND;
        let command = Command::new(cmd).args(vec![arg, $cmd]);
        $(
            command.args(String::try_from($args).unwrap_or(format!("Argument {:?} is causing an error", $args)).split_whitespace().map(str::to_string).collect::<Vec<String>>());
        )*
        command
            .stdout(Stdio::piped())
            .spawn()
            .expect(&format!("Failed to execute: {} {:?} - Input should be a valid command", $cmd, &[$($args),*]));
    }};
}

#[allow(unused_results)]
pub(crate) fn create_stream(mut child: Child) -> broadcast::Receiver<String> {
    let (tx, rx) = broadcast::channel(1000);
    let stdout = child.stdout.take().expect("Failed to take stdout");
    let stderr = child.stderr.take().expect("Failed to take stderr");

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    const CHILD_PROCESS_TIMEOUT: Duration = Duration::from_millis(10000);
    const MAX_CONSECUTIVE_NONE: usize = 5;

    tokio::spawn(async move {
        let mut stdout_none_count = 0;
        let mut stderr_none_count = 0;

        loop {
            tokio::select! {
                result = stdout_reader.next_line() => {
                    crate::error!("READ TO STDOUT: {:?}", result);
                    match result {
                        Ok(Some(line)) => {
                            stdout_none_count = 0;
                            if !line.is_empty() && tx.send(format!("stdout: {}", line)).is_err() {
                                    break;
                            }
                        }
                        Ok(None) => {
                            stdout_none_count += 1;
                            if stdout_none_count >= MAX_CONSECUTIVE_NONE {
                                crate::info!("Reached maximum consecutive None values for stdout");
                                break;
                            }
                        }
                        Err(e) => {
                            crate::error!("Error reading from stdout: {}", e);
                            break;
                        }
                    }
                }
                result = stderr_reader.next_line() => {
                    crate::error!("READ TO STDERR: {:?}", result);
                    match result {
                        Ok(Some(line)) => {
                            stderr_none_count = 0;
                            if !line.is_empty() && tx.send(format!("stderr: {}", line)).is_err() {
                                    break;
                                                            }
                        }
                        Ok(None) => {
                            stderr_none_count += 1;
                            if stderr_none_count >= MAX_CONSECUTIVE_NONE {
                                crate::info!("Reached maximum consecutive None values for stderr");
                                break;
                            }
                        }
                        Err(e) => {
                            crate::error!("Error reading from stderr: {}", e);
                            break;
                        }
                    }
                }
                _ = sleep(CHILD_PROCESS_TIMEOUT) => {
                    crate::info!("Child process timeout reached, waiting for process to exit");
                    if let Ok(status) = child.wait().await {
                        crate::info!("Child process exited with status: {}", status);
                    } else {
                        crate::error!("Failed to get child process exit status");
                    }
                    break;
                }
            }
        }

        let status = child
            .wait()
            .await
            .expect("Child process encountered an error...");
        println!("Child process ended with status: {}", status);
    });

    rx
}

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
                    eprintln!("Error sending message: {}", message);
                }
            }
        }
        Err(e) => {
            eprintln!("Error reading from {}: {}", source, e);
        }
    }
}

#[macro_export]
macro_rules! run_command {
    ($cmd:expr) => {{
        // Spawn child running process
        let child: Child = craft_child_process!($cmd);
        let pid = child.id().ok_or(Error::UnexpectedExit)?;
        let stream = create_stream(child);
        GadgetProcess::new(
            $cmd.to_string(),
            pid,
            Vec::new(),
            stream,
        )
    }};
    ($cmd:expr, $($args:expr),*) => {{
        // Spawn child running process
        let child = craft_child_process!($cmd,$($args),*);
        let pid = child.id().ok_or(Error::UnexpectedExit)?;
        let stream = create_stream(child);
        GadgetProcess::new(
            $cmd.to_string(),
            pid,
            Vec::new(),
            stream,
        )
    }};
}
