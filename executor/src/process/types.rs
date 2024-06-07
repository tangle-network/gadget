use crate::process::utils::*;
use crate::{craft_child_process, run_command, OS_COMMAND};
use failure::format_err;
use nix::libc::pid_t;
use nix::sys::signal;
use nix::sys::signal::Signal;
use procfs::process::Process;
use procfs::ProcError;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;
use tokio::io::{BufReader, Lines};
pub use tokio::process::Child;
use tokio::time::timeout;

/// A Process spawned by gadget-executor, running some service or command(s)
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GadgetProcess {
    /// The command executed
    pub(crate) command: String,
    /// The name of the process itself
    pub(crate) process_name: String,
    /// Process ID
    pub(crate) pid: u32,
    /// History of output from process for reviewing/tracking progress
    pub(crate) output: Vec<String>,
    /// Stream for output from child process
    #[serde(skip_serializing, skip_deserializing)]
    pub(crate) stream: Option<Lines<BufReader<tokio::process::ChildStdout>>>,
}

impl GadgetProcess {
    pub fn new(
        command: String,
        pid: Option<u32>,
        output: Vec<String>,
        stream: Lines<BufReader<tokio::process::ChildStdout>>,
    ) -> Result<GadgetProcess, Box<dyn Error>> {
        let process_name = Process::new(pid.ok_or("PID does not exist")? as i32)?
            .status()?
            .name;
        let pid = pid.ok_or("PID does not exist")?;
        Ok(GadgetProcess {
            command,
            process_name,
            pid,
            output,
            stream: Some(stream),
        })
    }

    /// Read output from this GadgetProcess, returning None if there was no output
    pub async fn read(&mut self) -> ProcessOutput {
        let messages = Vec::new();
        if let Some(stream) = &mut self.stream {
            // Read lines until we time out, meaning we are still waiting for output - continue for now
            loop {
                let read_result = timeout(Duration::from_millis(50), stream.next_line()).await;
                match read_result {
                    Ok(output) => {
                        if output.is_err() {
                            // TODO: Error logging
                            println!("{} encountered read error", self.process_name.clone());
                        } else {
                            let inbound_message = output.unwrap();
                            match inbound_message {
                                None => {
                                    // Stream is completed - process is finished
                                    println!(
                                        "{} : STREAM COMPLETED - ENDING",
                                        self.process_name.clone()
                                    );
                                    // TODO: Log
                                    return ProcessOutput::Exhausted(messages);
                                }
                                Some(streamed_output) => {
                                    // We received output from child process
                                    // TODO: Log
                                    println!(
                                        "{} : MESSAGE LOG : {}",
                                        self.process_name.clone(),
                                        streamed_output.clone()
                                    );
                                    self.output.push(streamed_output);
                                }
                            }
                        }
                    }
                    Err(timeout) => {
                        // TODO: Log
                        println!(
                            "{} read attempt timed out after {}, continuing...",
                            self.process_name.clone(),
                            timeout
                        );
                        break;
                    }
                }
            }

            if messages.is_empty() {
                ProcessOutput::Waiting
            } else {
                ProcessOutput::Output(messages)
            }
        } else {
            // TODO: Error logging
            println!("{} encountered read error", self.process_name.clone());
            ProcessOutput::Waiting
        }
    }

    /// Restart a GadgetProcess, killing the previously running process if it exists. Returns the new GadgetProcess
    pub(crate) async fn restart_process(&mut self) -> Result<GadgetProcess, Box<dyn Error>> {
        // Kill current process running this command
        // let status = Process::new(self.pid as i32)?.status()?;
        match Process::new(self.pid as i32) {
            Ok(process) => {
                let status = process.status()?;
                if status.name == self.process_name {
                    self.kill()?;
                }
            }
            Err(err) => {
                if matches!(err, ProcError::NotFound(_)) {
                    // No need to worry, the previously running process died
                    // TODO: Log
                    println!("LOG : Process restart attempt yielded error {:?}", err);
                } else {
                    println!("ERROR : Process restart attempt yielded error {:?}", err);
                    return Err(err.into());
                }
            }
        }
        run_command!(&self.command.clone())
    }

    /// Checks the status of this GadgetProcess
    pub(crate) fn status(&self) -> Result<Status, Box<dyn Error>> {
        match Process::new(self.pid as i32) {
            Ok(process) => {
                let status = process.stat()?;
                Ok(Status::from(status.state))
            }
            Err(err) => {
                if matches!(err, ProcError::NotFound(_)) {
                    // If it isn't found, then the process died
                    Ok(Status::Dead)
                } else {
                    // If there is an error, then
                    println!(
                        "ERROR : Status check on process {} yielded {:?}",
                        self.pid, err
                    );
                    Err(err.into())
                }
            }
        }
    }

    /// Gets process name by PID
    #[allow(dead_code)]
    pub(crate) fn get_name(&self) -> Result<String, Box<dyn Error>> {
        let status = Process::new(self.pid as i32)?.status()?;
        Ok(status.name)
    }

    /// Terminates the process depicted by this GadgetProcess - will fail if the PID is now being reused
    pub(crate) fn kill(&self) -> Result<(), Box<dyn Error>> {
        let running_process = Process::new(self.pid as i32)?.status()?.name;
        if running_process == self.process_name {
            Ok(signal::kill(
                nix::unistd::Pid::from_raw(self.pid as pid_t),
                Signal::SIGTERM,
            )?)
        } else {
            Err(Box::from(format_err!(
                "Expected {} and found {} running instead - process termination aborted",
                self.process_name,
                running_process
            )))
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum ProcessOutput {
    /// Normal collection of output lines from a given process
    Output(Vec<String>),
    /// Output lines from a given process that successfully completed
    Exhausted(Vec<String>),
    /// No output - still waiting for next output from process
    Waiting,
}

#[derive(Debug)]
pub(crate) enum Status {
    /// Process is running or able to run
    Active,
    /// Stopped process
    Inactive,
    /// Sleeping process, either waiting for resources or a signal
    Sleeping,
    /// Zombie process
    Dead,
    /// Other or invalid status - if this occurs, something is likely wrong
    Unknown,
}

impl From<char> for Status {
    fn from(value: char) -> Status {
        match value {
            'R' => Status::Active,
            'S' | 'D' => Status::Sleeping,
            'T' => Status::Inactive,
            'Z' | 'X' | 'x' => Status::Dead,
            _ => Status::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GadgetInstructionData {
    pub(crate) commands: Vec<CommandData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct CommandData {
    pub(crate) name: String,
    pub(crate) command: CommandOrSequence,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum CommandOrSequence {
    Command(String),
    Sequence(Vec<String>),
}
