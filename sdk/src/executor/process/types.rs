use super::error::Error;
use crate::executor::process::utils::*;
use crate::executor::OS_COMMAND;
use crate::info;
use crate::{craft_child_process, run_command};
use nix::libc::pid_t;
use nix::sys::signal;
use nix::sys::signal::Signal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ffi::OsString;
use std::time::Duration;
use sysinfo::ProcessStatus::{
    Dead, Idle, LockBlocked, Parked, Run, Sleep, Stop, Tracing, UninterruptibleDiskSleep, Unknown,
    Wakekill, Waking, Zombie,
};
use sysinfo::{Pid, ProcessStatus, System};
pub use tokio::process::Child;
use tokio::sync::broadcast;

const DEFAULT_READ_TIMEOUT: u64 = 60; // seconds

/// A process spawned by gadget-executor, running some service or command(s)
#[derive(Serialize, Deserialize, Debug)]
pub struct GadgetProcess {
    /// The command executed
    pub command: String,
    /// The name of the process itself
    pub process_name: OsString,
    /// Process ID
    #[serde(serialize_with = "serialize_pid", deserialize_with = "deserialize_pid")]
    pub pid: Pid,
    /// History of output from process for reviewing/tracking progress
    pub output: Vec<String>,
    /// [Stream](broadcast::Receiver) for output from child process
    #[serde(skip_serializing, skip_deserializing)]
    pub stream: Option<broadcast::Receiver<String>>,
}

fn serialize_pid<S>(pid: &Pid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u32(pid.as_u32())
}

fn deserialize_pid<'de, D>(deserializer: D) -> Result<Pid, D::Error>
where
    D: Deserializer<'de>,
{
    let value = u32::deserialize(deserializer)?;
    Ok(Pid::from_u32(value))
}

impl GadgetProcess {
    pub fn new(
        command: String,
        pid: u32,
        output: Vec<String>,
        stream: broadcast::Receiver<String>,
    ) -> Result<GadgetProcess, Error> {
        let s = System::new_all();
        let pid = Pid::from_u32(pid);
        let process_name = s
            .process(pid)
            .ok_or(Error::ProcessNotFound(pid))?
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

    /// Resubscribe to this [GadgetProcess]'s output [stream](broadcast::Receiver).
    ///
    /// Essentially clones the output stream of this [GadgetProcess].
    ///
    /// # Errors
    /// - If the [stream](broadcast::Receiver) has died
    /// - If the [GadgetProcess] has been deserialized and the [stream](broadcast::Receiver) is None
    pub fn resubscribe(&self) -> Result<broadcast::Receiver<String>, Box<dyn Error>> {
        match &self.stream {
            Some(stream) => Ok(stream.resubscribe()),
            None => Err(Box::from(format_err!(
                "Failed to resubscribe, stream is None"
            ))),
        }
    }

    /// Continually reads output from this [GadgetProcess], eventually returning a [ProcessOutput].
    ///
    /// Will loop and wait for output from the process, returning upon timeout or completion
    /// of output [stream](broadcast::Receiver).
    pub async fn read_until_timeout(&mut self, timeout: u64) -> ProcessOutput {
        let mut messages = Vec::new();
        if let Some(stream) = &mut self.stream {
            // Read lines until we time out, meaning we are still waiting for output - continue for now
            loop {
                let read_result =
                    tokio::time::timeout(Duration::from_secs(timeout), stream.recv()).await;
                match read_result {
                    Ok(output) => {
                        match output {
                            Err(e) => {
                                info!("{} ended with: {}", self.process_name.to_string_lossy(), e);
                                return ProcessOutput::Exhausted(messages);
                            }
                            Ok(inbound_message) => {
                                if inbound_message.is_empty() {
                                    // Stream is completed - process is finished
                                    info!(
                                        "{} : STREAM COMPLETED - ENDING",
                                        self.process_name.to_string_lossy()
                                    );
                                    return ProcessOutput::Exhausted(messages);
                                } else {
                                    // We received output from child process
                                    info!(
                                        "{} : MESSAGE LOG : {}",
                                        self.process_name.to_string_lossy(),
                                        inbound_message
                                    );
                                    messages.push(inbound_message.clone());
                                    self.output.push(inbound_message);
                                }
                            }
                        }
                    }
                    Err(_timeout) => {
                        info!(
                            "{:?} read attempt timed out after {} second(s), continuing...",
                            self.process_name.clone(),
                            timeout
                        );
                        break;
                    }
                }
            }
            info!("EXECUTOR READ LOOP ENDED");

            if messages.is_empty() {
                ProcessOutput::Waiting
            } else {
                ProcessOutput::Output(messages)
            }
        } else {
            info!(
                "{} encountered read error",
                self.process_name.to_string_lossy()
            );
            ProcessOutput::Waiting
        }
    }

    /// Continually reads output from this [GadgetProcess], eventually returning a [ProcessOutput].
    /// Will loop and wait for output from the process, returns early if no output is received
    /// for a default timeout period of 1 second.
    pub(crate) async fn read_until_default_timeout(&mut self) -> ProcessOutput {
        self.read_until_timeout(DEFAULT_READ_TIMEOUT).await
    }

    /// Continually reads output from this [GadgetProcess], eventually returning a [ProcessOutput].
    /// Will loop and wait for output to contain the specified substring.
    pub async fn read_until_receiving_string(&mut self, substring: String) -> ProcessOutput {
        let mut messages = Vec::new();
        if let Some(stream) = &mut self.stream {
            // Read lines until we receive the desired substring
            loop {
                let read_result = stream.recv().await;
                match read_result {
                    Ok(output) => {
                        let inbound_message = output;
                        if inbound_message.is_empty() {
                            // Stream is completed - process is finished
                            info!(
                                "{} : STREAM COMPLETED - ENDING",
                                self.process_name.to_string_lossy()
                            );
                            return ProcessOutput::Exhausted(messages);
                        } else {
                            // We received output from child process
                            info!(
                                "{} : MESSAGE LOG : {}",
                                self.process_name.to_string_lossy(),
                                inbound_message.clone()
                            );
                            messages.push(inbound_message.clone());
                            self.output.push(inbound_message.clone());
                            if inbound_message.contains(&substring) {
                                // We should now return with the output
                                return ProcessOutput::Output(messages);
                            }
                        }
                    }
                    Err(err) => {
                        info!(
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
        info!(
            "{} encountered read error",
            self.process_name.to_string_lossy()
        );
        ProcessOutput::Waiting
    }

    /// Restart a [GadgetProcess], killing the previously running process if it exists. Returns the new [GadgetProcess]
    pub(crate) async fn restart_process(&mut self) -> Result<GadgetProcess, Error> {
        // Kill current process running this command
        let s = System::new_all();
        match s.process(self.pid) {
            Some(process) => {
                if process.name() == self.process_name {
                    self.kill()?;
                }
            }
            None => {
                // No need to worry, the previously running process died
                info!(
                    "LOG : Process restart attempt found no process for PID {:?}",
                    self.pid
                );
            }
        }
        run_command!(&self.command.clone())
    }

    /// Checks the status of this [GadgetProcess]
    pub(crate) fn status(&self) -> Result<Status, Error> {
        let s = System::new_all();
        match s.process(self.pid) {
            Some(process) => Status::from(process.status()),
            None => {
                // If it isn't found, then the process died
                Status::Dead
            }
        }
    }

    /// Gets process name by PID
    #[allow(dead_code)]
    pub(crate) fn get_name(&self) -> Result<OsString, Error> {
        let s = System::new_all();
        let name = s
            .process(self.pid)
            .ok_or(Error::ProcessNotFound(self.pid))?
            .name();
        Ok(name.into())
    }

    /// Terminates the process depicted by this [GadgetProcess] - will fail if the PID is now being reused
    pub(crate) fn kill(&self) -> Result<(), Error> {
        let running_process = self.get_name()?;
        if running_process != self.process_name {
            return Err(Error::ProcessMismatch(
                self.process_name.to_string_lossy().into_owned(),
                running_process.to_string_lossy().into_owned(),
            ));
        }

        signal::kill(
            nix::unistd::Pid::from_raw(self.pid.as_u32() as pid_t),
            Signal::SIGTERM,
        )
        .map_err(Error::KillFailed)?;

        Ok(())
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ProcessOutput {
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
