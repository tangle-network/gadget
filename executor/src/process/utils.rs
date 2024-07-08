pub use futures::{FutureExt, StreamExt};
pub use std::process::Stdio;
pub use tokio::io::Lines;
pub use tokio::io::{AsyncBufReadExt, BufReader};
pub use tokio::process::{Child, Command};

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

pub(crate) fn create_stream(mut child: Child) -> Lines<BufReader<tokio::process::ChildStdout>> {
    // Create stream to read output
    let stdout = child
        .stdout
        .take()
        .expect("Failed to take stdout handle from child...");
    let reader = BufReader::new(stdout).lines();

    // Run in Tokio runtime to ensure it stays alive
    tokio::spawn(async move {
        let status = child
            .wait()
            .await
            .expect("Child process encountered an error...");
        println!("Child process ended with status: {}", status)
    });

    // Return stream
    reader
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
