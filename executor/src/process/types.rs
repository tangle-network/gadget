use crate::process::utils::*;
use crate::{craft_child_process, run_command, OS_COMMAND};
use failure::format_err;
use nix::libc::pid_t;
use nix::sys::signal;
use nix::sys::signal::Signal;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::ffi::OsString;
use std::time::Duration;
use sysinfo::ProcessStatus::{
    Dead, Idle, LockBlocked, Parked, Run, Sleep, Stop, Tracing, UninterruptibleDiskSleep, Unknown,
    Wakekill, Waking, Zombie,
};
use sysinfo::{Pid, ProcessStatus, System};
use tokio::io::{BufReader, Lines};
pub use tokio::process::Child;

const DEFAULT_READ_TIMEOUT: u64 = 1000;

/// A Process spawned by gadget-executor, running some service or command(s)
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GadgetProcess {
    /// The command executed
    pub(crate) command: String,
    /// The name of the process itself
    pub(crate) process_name: OsString,
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
        let s = System::new_all();
        let pid = pid.ok_or("No PID found")?;
        let process_name = s
            .process(Pid::from_u32(pid))
            .ok_or(format!("Process {pid} doesn't exist"))?
            .name()
            .to_os_string();
        Ok(GadgetProcess {
            command,
            process_name,
            pid,
            output,
            stream: Some(stream),
        })
    }

    /// Continually reads output from this GadgetProcess, eventually returning a ProcessOutput.
    /// Will loop and wait for output from the process, returning upon timeout or completion
    /// of output stream.
    pub async fn read_until_timeout(&mut self, timeout: u64) -> ProcessOutput {
        let mut messages = Vec::new();
        if let Some(stream) = &mut self.stream {
            // Read lines until we time out, meaning we are still waiting for output - continue for now
            loop {
                let read_result =
                    tokio::time::timeout(Duration::from_millis(timeout), stream.next_line()).await;
                match read_result {
                    Ok(output) => {
                        if output.is_err() {
                            // TODO: Error logging
                            println!(
                                "{} encountered read error",
                                self.process_name.to_string_lossy()
                            );
                        } else {
                            let inbound_message = output.unwrap();
                            match inbound_message {
                                None => {
                                    // Stream is completed - process is finished
                                    println!(
                                        "{} : STREAM COMPLETED - ENDING",
                                        self.process_name.to_string_lossy()
                                    );
                                    // TODO: Log
                                    return ProcessOutput::Exhausted(messages);
                                }
                                Some(streamed_output) => {
                                    // We received output from child process
                                    // TODO: Log
                                    println!(
                                        "{} : MESSAGE LOG : {}",
                                        self.process_name.to_string_lossy(),
                                        streamed_output.clone()
                                    );
                                    messages.push(streamed_output.clone());
                                    self.output.push(streamed_output);
                                }
                            }
                        }
                    }
                    Err(_timeout) => {
                        // TODO: Log
                        // println!(
                        //     "{} read attempt timed out after {}, continuing...",
                        //     self.process_name.clone(),
                        //     timeout
                        // );
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
            println!(
                "{} encountered read error",
                self.process_name.to_string_lossy()
            );
            ProcessOutput::Waiting
        }
    }

    /// Continually reads output from this GadgetProcess, eventually returning a ProcessOutput.
    /// Will loop and wait for output from the process, returns early if no output is received
    /// for a default timeout period of 1 second.
    pub(crate) async fn read_until_default_timeout(&mut self) -> ProcessOutput {
        self.read_until_timeout(DEFAULT_READ_TIMEOUT).await
    }

    /// Continually reads output from this GadgetProcess, eventually returning a ProcessOutput.
    /// Will loop and wait for output to contain the specified substring.
    pub async fn read_until_receiving_string(&mut self, substring: String) -> ProcessOutput {
        let mut messages = Vec::new();
        if let Some(stream) = &mut self.stream {
            // Read lines until we receive the desired substring
            loop {
                let read_result = stream.next_line().await;
                match read_result {
                    Ok(output) => {
                        match output {
                            None => {
                                // Stream is completed - process is finished
                                println!(
                                    "{} : STREAM COMPLETED - ENDING",
                                    self.process_name.to_string_lossy()
                                );
                                return ProcessOutput::Exhausted(messages);
                            }
                            Some(streamed_output) => {
                                // We received output from child process
                                // TODO: Log
                                println!(
                                    "{} : MESSAGE LOG : {}",
                                    self.process_name.to_string_lossy(),
                                    streamed_output.clone()
                                );
                                messages.push(streamed_output.clone());
                                self.output.push(streamed_output.clone());
                                if streamed_output.contains(&substring) {
                                    // We should now return with the output
                                    return ProcessOutput::Output(messages);
                                }
                            }
                        }
                    }
                    Err(err) => {
                        // TODO: Log
                        println!(
                            "{} read attempt failed: {}",
                            self.process_name.to_string_lossy(),
                            err
                        );
                        break;
                    }
                }
            }
        }
        // Reaching this point means there was some sort of error - we never got the substring
        // TODO: Error logging
        println!(
            "{} encountered read error",
            self.process_name.to_string_lossy()
        );
        ProcessOutput::Waiting
    }

    /// Restart a GadgetProcess, killing the previously running process if it exists. Returns the new GadgetProcess
    pub(crate) async fn restart_process(&mut self) -> Result<GadgetProcess, Box<dyn Error>> {
        // Kill current process running this command
        let s = System::new_all();
        match s.process(Pid::from_u32(self.pid)) {
            Some(process) => {
                if process.name() == self.process_name {
                    self.kill()?;
                }
            }
            None => {
                // No need to worry, the previously running process died
                // TODO: Log
                println!(
                    "LOG : Process restart attempt found no process for PID {:?}",
                    self.pid
                );
            }
        }
        run_command!(&self.command.clone())
    }

    /// Checks the status of this GadgetProcess
    pub(crate) fn status(&self) -> Result<Status, Box<dyn Error>> {
        let s = System::new_all();
        match s.process(Pid::from_u32(self.pid)) {
            Some(process) => Ok(Status::from(process.status())),
            None => {
                // If it isn't found, then the process died
                Ok(Status::Dead)
            }
        }
    }

    /// Gets process name by PID
    #[allow(dead_code)]
    pub(crate) fn get_name(&self) -> Result<OsString, Box<dyn Error>> {
        let s = System::new_all();
        let name = s
            .process(Pid::from_u32(self.pid))
            .ok_or(format!("Process {} doesn't exist", self.pid))?
            .name();
        Ok(name.into())
    }

    /// Terminates the process depicted by this GadgetProcess - will fail if the PID is now being reused
    pub(crate) fn kill(&self) -> Result<(), Box<dyn Error>> {
        let running_process = self.get_name()?;
        if running_process == self.process_name {
            Ok(signal::kill(
                nix::unistd::Pid::from_raw(self.pid as pid_t),
                Signal::SIGTERM,
            )?)
        } else {
            Err(Box::from(format_err!(
                "Expected {} and found {} running instead - process termination aborted",
                self.process_name.to_string_lossy(),
                running_process.to_string_lossy()
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
    Unknown(String),
}

impl From<ProcessStatus> for Status {
    fn from(value: ProcessStatus) -> Status {
        match value {
            Run | Waking => Status::Active,
            Sleep | UninterruptibleDiskSleep | Parked | LockBlocked | Wakekill => Status::Sleeping,
            Stop | Tracing | Idle => Status::Inactive,
            Dead | Zombie => Status::Dead,
            Unknown(code) => Status::Unknown(format!("Unknown with code {code}")),
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
