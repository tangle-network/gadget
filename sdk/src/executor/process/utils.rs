pub use futures::{FutureExt, StreamExt};
pub use std::process::Stdio;
pub use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, AsyncRead, ReadBuf};
use tokio::macros::support::poll_fn;
pub use tokio::process::{Child, Command};
use tokio::sync::broadcast;

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

    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = stdout_reader.next_line() => {
                    if let Ok(Some(line)) = result {
                        if tx.send(format!("stdout: {}", line)).is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                result = stderr_reader.next_line() => {
                    if let Ok(Some(line)) = result {
                        if tx.send(format!("stderr: {}", line)).is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ = child.wait() => {
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
        let pid = child.id().clone();
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
        let pid = child.id().clone();
        let stream = create_stream(child);
        GadgetProcess::new(
            $cmd.to_string(),
            pid,
            Vec::new(),
            stream,
        )
    }};
}
