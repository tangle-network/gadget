use super::error::Error;
use crate::utils::{
    create_stream, get_process_info, ChildInfo, Command, Stdio, OS_ARG, OS_COMMAND,
};
use crate::{craft_child_process, run_command};
use nix::sys::signal;
use nix::sys::signal::Signal;
use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::ffi::OsString;
use std::time::Duration;
use sysinfo::{Pid, ProcessStatus, System};
use tokio::sync::broadcast;
use tokio::sync::oneshot;

const DEFAULT_READ_TIMEOUT: u64 = 60; // seconds

/// A process spawned by gadget-executor, running some service or command(s)
#[derive(Debug, Clone)]
pub struct GadgetProcess {
    /// The command executed
    pub command: String,
    /// The name of the process itself
    pub process_name: OsString,
    /// Process ID
    pub pid: Option<Pid>,
    /// [Stream](broadcast::Receiver) for output from child process
    #[serde(skip)]
    pub stream: Option<broadcast::Receiver<String>>,
    /// History of output from process for reviewing/tracking progress
    #[serde(skip)]
    pub output: Option<broadcast::Sender<String>>,
    /// Status of the process
    pub status: Status,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializedGadgetProcess {
    /// The command executed
    pub command: String,
    /// The name of the process itself
    pub process_name: OsString,
    /// Process ID
    #[serde(serialize_with = "serialize_pid", deserialize_with = "deserialize_pid")]
    pub pid: Option<Pid>,
    /// Status of the process
    pub status: Status,
}

impl From<GadgetProcess> for SerializedGadgetProcess {
    fn from(process: GadgetProcess) -> Self {
        SerializedGadgetProcess {
            command: process.command,
            process_name: process.process_name,
            pid: process.pid,
            status: process.status,
        }
    }
}

impl From<SerializedGadgetProcess> for GadgetProcess {
    fn from(process: SerializedGadgetProcess) -> Self {
        GadgetProcess {
            command: process.command,
            process_name: process.process_name,
            pid: process.pid,
            stream: None,
            output: None,
            status: process.status,
        }
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn serialize_pid<S>(pid: &Option<Pid>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match pid {
        Some(pid) => serializer.serialize_u32(pid.as_u32()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_pid<'de, D>(deserializer: D) -> Result<Option<Pid>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<u32>::deserialize(deserializer)?;
    match value {
        Some(value) => Ok(Some(Pid::from_u32(value))),
        None => Ok(None),
    }
}

impl GadgetProcess {
    pub async fn new(command: String) -> Result<Self, Error> {
        let child_info = run_command!(&command).await?;
        let (tx, _) = broadcast::channel(100);
        
        Ok(Self {
            pid: Some(Pid::from(child_info.pid)),
            process_name: command.clone().into(),
            command,
            status: Status::Active,
            output: Some(tx),
            stream: None,
        })
    }

    pub async fn kill(&mut self) -> Result<(), Error> {
        if let Some(pid) = self.pid {
            let _ = nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid.as_u32() as i32),
                nix::sys::signal::Signal::SIGTERM,
            );
            self.status = Status::Stopped;
        }
        Ok(())
    }

    pub fn get_output(&self) -> Option<broadcast::Receiver<String>> {
        self.output.as_ref().map(|tx| tx.subscribe())
    }

    /// Resubscribe to this [`GadgetProcess`]'s output [stream](broadcast::Receiver).
    ///
    /// Essentially clones the output stream of this [`GadgetProcess`].
    ///
    /// # Errors
    /// - If the [stream](broadcast::Receiver) has died
    /// - If the [`GadgetProcess`] has been deserialized and the [stream](broadcast::Receiver) is None
    pub fn resubscribe(&self) -> Result<broadcast::Receiver<String>, Error> {
        match &self.stream {
            Some(stream) => Ok(stream.resubscribe()),
            None => Err(Error::StreamError(self.pid.unwrap_or(Pid::from_u32(0)), "No output stream available".to_string())),
        }
    }

    /// Continually reads output from this [`GadgetProcess`], eventually returning a [`ProcessOutput`].
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
                                gadget_logging::debug!(
                                    "{} ended with: {}",
                                    self.process_name.to_string_lossy(),
                                    e
                                );
                                return ProcessOutput::Exhausted(messages);
                            }
                            Ok(inbound_message) => {
                                if inbound_message.is_empty() {
                                    // Stream is completed - process is finished
                                    gadget_logging::debug!(
                                        "{} : STREAM COMPLETED - ENDING",
                                        self.process_name.to_string_lossy()
                                    );
                                    return ProcessOutput::Exhausted(messages);
                                }
                                // We received output from child process
                                gadget_logging::debug!(
                                    "{} : MESSAGE LOG : {}",
                                    self.process_name.to_string_lossy(),
                                    inbound_message
                                );
                                messages.push(inbound_message.clone());
                                if let Some(output) = &mut self.output {
                                    output.send(inbound_message.clone()).unwrap();
                                }
                            }
                        }
                    }
                    Err(_timeout) => {
                        gadget_logging::info!(
                            "{:?} read attempt timed out after {} second(s), continuing...",
                            self.process_name.clone(),
                            timeout
                        );
                        break;
                    }
                }
            }
            gadget_logging::debug!("EXECUTOR READ LOOP ENDED");

            if messages.is_empty() {
                ProcessOutput::Waiting
            } else {
                ProcessOutput::Output(messages)
            }
        } else {
            gadget_logging::warn!(
                "{} encountered read error",
                self.process_name.to_string_lossy()
            );
            ProcessOutput::Waiting
        }
    }

    /// Continually reads output from this [`GadgetProcess`], eventually returning a [`ProcessOutput`].
    /// Will loop and wait for output from the process, returns early if no output is received
    /// for a default timeout period of 1 second.
    pub(crate) async fn read_until_default_timeout(&mut self) -> ProcessOutput {
        self.read_until_timeout(DEFAULT_READ_TIMEOUT).await
    }

    /// Continually reads output from this [`GadgetProcess`], eventually returning a [`ProcessOutput`].
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
                            gadget_logging::debug!(
                                "{} : STREAM COMPLETED - ENDING",
                                self.process_name.to_string_lossy()
                            );
                            return ProcessOutput::Exhausted(messages);
                        }
                        // We received output from child process
                        gadget_logging::debug!(
                            "{} : MESSAGE LOG : {}",
                            self.process_name.to_string_lossy(),
                            inbound_message.clone()
                        );
                        messages.push(inbound_message.clone());
                        if let Some(output) = &mut self.output {
                            output.send(inbound_message.clone()).unwrap();
                        }
                        if inbound_message.contains(&substring) {
                            // We should now return with the output
                            return ProcessOutput::Output(messages);
                        }
                    }
                    Err(err) => {
                        gadget_logging::warn!(
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
        gadget_logging::warn!(
            "{} encountered read error",
            self.process_name.to_string_lossy()
        );
        ProcessOutput::Waiting
    }

    /// Restart a [`GadgetProcess`], killing the previously running process if it exists. Returns the new [`GadgetProcess`]
    pub(crate) fn restart_process(&mut self) -> Result<GadgetProcess, Error> {
        // Kill current process running this command
        let s = System::new_all();
        match s.process(self.pid.unwrap_or(Pid::from_u32(0))) {
            Some(process) => {
                if process.name() == self.process_name {
                    self.kill()?;
                }
            }
            None => {
                // No need to worry, the previously running process died
                gadget_logging::warn!(
                    "Process restart attempt found no process for PID {:?}",
                    self.pid
                );
            }
        }
        let command = run_command!(&self.command.clone());
        command.await
    }

    /// Checks the status of this [`GadgetProcess`]
    pub(crate) fn status(&self) -> Status {
        let s = System::new_all();
        match s.process(self.pid.unwrap_or(Pid::from_u32(0))) {
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
            .process(self.pid.unwrap_or(Pid::from_u32(0)))
            .ok_or(Error::ProcessNotFound(self.pid.unwrap_or(Pid::from_u32(0))))?
            .name();
        Ok(name.into())
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        let command = run_command!(&self.command).await?;
        self.output = Some(command.tx);
        Ok(())
    }

    pub fn get_output(&self) -> Result<broadcast::Receiver<String>, Error> {
        match &self.output {
            Some(tx) => Ok(tx.subscribe()),
            None => Err(Error::StreamError(
                self.pid.unwrap_or(Pid::from_u32(0)),
                "No output stream available".to_string()
            )),
        }
    }
}

#[derive(Debug)]
pub enum ProcessOutput {
    /// Normal collection of output lines from a given process
    Output(Vec<String>),
    /// Output lines from a given process that successfully completed
    Exhausted(Vec<String>),
    /// No output - still waiting for next output from process
    Waiting,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Status {
    Active,
    Sleeping,
    Dead,
    Stopped,
}

impl From<ProcessStatus> for Status {
    fn from(value: ProcessStatus) -> Self {
        match value {
            ProcessStatus::Run | ProcessStatus::Waking => Status::Active,
            ProcessStatus::Sleep
            | ProcessStatus::UninterruptibleDiskSleep
            | ProcessStatus::Parked
            | ProcessStatus::LockBlocked
            | ProcessStatus::Wakekill => Status::Sleeping,
            ProcessStatus::Stop | ProcessStatus::Tracing | ProcessStatus::Idle => Status::Stopped,
            ProcessStatus::Dead | ProcessStatus::Zombie => Status::Dead,
            ProcessStatus::Unknown(code) => Status::Stopped,
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
