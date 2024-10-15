pub use futures::{FutureExt, StreamExt};
pub use std::process::Stdio;
pub use tokio::io::{AsyncBufReadExt, BufReader};
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
    // Create a broadcast channel
    let (tx, rx) = broadcast::channel(100);

    // Create stream to read output
    let stdout = child
        .stdout
        .take()
        .expect("Failed to take stdout handle from child...");
    let mut reader = BufReader::new(stdout).lines();

    // Spawn a task to read from the process's output and send to the broadcast channel
    tokio::spawn(async move {
        while let Some(line) = reader
            .next_line()
            .await
            .expect("Failed to read line from stdout")
        {
            if tx.send(line).is_err() {
                // If there are no active receivers, stop the task
                break;
            }
        }
        let status = child
            .wait()
            .await
            .expect("Child process encountered an error...");
        println!("Child process ended with status: {}", status);
    });

    // Return the receiver
    rx
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
