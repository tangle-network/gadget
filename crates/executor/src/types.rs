use super::error::Error;
use crate::run_command;
use crate::utils::create_stream;
use nix::sys::signal;
use nix::sys::signal::Signal;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::time::Duration;
use sysinfo::{Pid, ProcessStatus, System};
use tokio::sync::broadcast;

const DEFAULT_READ_TIMEOUT: u64 = 60; // seconds

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Status {
    Active,
    Sleeping,
    Dead,
    Stopped,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GadgetProcess {
    pub command: String,
    pub process_name: OsString,
    #[serde(skip)]
    pub pid: Option<Pid>,
    #[serde(skip)]
    pub stream: Option<broadcast::Receiver<String>>,
    #[serde(skip)]
    pub output: Option<broadcast::Sender<String>>,
    pub status: Status,
}

impl GadgetProcess {
    pub async fn new(command: String) -> Result<Self, Error> {
        let child_info = run_command!(&command).await?;
        let output = Some(child_info.tx);
        let stream = output
            .as_ref()
            .map(tokio::sync::broadcast::Sender::subscribe);
        Ok(Self {
            command,
            process_name: OsString::from(""), // Will be updated when process info is available
            pid: Some(Pid::from(child_info.pid as usize)),
            stream,
            output,
            status: Status::Active,
        })
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        let child_info = run_command!(&self.command).await?;
        self.pid = Some(Pid::from(child_info.pid as usize));
        self.output = Some(child_info.tx);
        self.stream = self
            .output
            .as_ref()
            .map(tokio::sync::broadcast::Sender::subscribe);
        Ok(())
    }

    pub fn kill(&mut self) -> Result<(), Error> {
        signal::kill(
            nix::unistd::Pid::from_raw(
                self.pid
                    .ok_or_else(|| Error::ServiceNotFound("PID was none".to_string()))?
                    .as_u32()
                    .cast_signed(),
            ),
            Signal::SIGTERM,
        )
        .map_err(|e| Error::KillFailed(e, "Kill failed".to_string()))?;

        self.status = self.status();

        Ok(())
    }

    pub fn get_output(&self) -> Result<broadcast::Receiver<String>, Error> {
        match &self.output {
            Some(tx) => Ok(tx.subscribe()),
            None => Err(Error::StreamError(
                self.pid.unwrap_or(Pid::from(0)),
                "No output stream available".to_string(),
            )),
        }
    }

    pub async fn restart_process(&mut self) -> Result<GadgetProcess, Error> {
        if self.pid.is_some() {
            self.kill()?;
        }
        let command = self.command.clone();
        GadgetProcess::new(command).await
    }

    pub fn resubscribe(&self) -> Result<broadcast::Receiver<String>, Error> {
        match &self.stream {
            Some(stream) => Ok(stream.resubscribe()),
            None => Err(Error::StreamError(
                self.pid.unwrap_or(Pid::from(0)),
                "No output stream available".to_string(),
            )),
        }
    }

    pub async fn read_until_timeout(&mut self, timeout: u64) -> Result<ProcessOutput, Error> {
        let mut messages = Vec::new();
        gadget_logging::info!(
            "{} : PROCESS STATUS : {:?}",
            self.process_name.to_string_lossy(),
            self.status
        );
        if self.status != Status::Active {
            gadget_logging::info!(
                "{} : PROCESS IS NOT ACTIVE",
                self.process_name.to_string_lossy()
            );
            return Ok(ProcessOutput::Exhausted(messages));
        }

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
                                return Ok(ProcessOutput::Exhausted(messages));
                            }
                            Ok(inbound_message) => {
                                if inbound_message.trim().is_empty() {
                                    // Stream is completed - process is finished
                                    gadget_logging::debug!(
                                        "{} : STREAM COMPLETED - ENDING",
                                        self.process_name.to_string_lossy()
                                    );
                                    return Ok(ProcessOutput::Exhausted(messages));
                                }
                                // We received output from child process
                                gadget_logging::debug!(
                                    "{} : MESSAGE LOG : {}",
                                    self.process_name.to_string_lossy(),
                                    inbound_message
                                );
                                messages.push(inbound_message.clone());
                                if inbound_message.contains("nonexistent: command not found") {
                                    return Err(Error::InvalidCommand(
                                        "Command nonexistent".to_string(),
                                    ));
                                } else if let Some(output) = &mut self.output {
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
                Ok(ProcessOutput::Waiting)
            } else {
                Ok(ProcessOutput::Output(messages))
            }
        } else {
            gadget_logging::warn!(
                "{} encountered read error",
                self.process_name.to_string_lossy()
            );
            Ok(ProcessOutput::Waiting)
        }
    }

    pub(crate) async fn read_until_default_timeout(&mut self) -> Result<ProcessOutput, Error> {
        self.read_until_timeout(DEFAULT_READ_TIMEOUT).await
    }

    pub async fn read_until_receiving_string(
        &mut self,
        substring: String,
    ) -> Result<ProcessOutput, Error> {
        let mut messages = Vec::new();
        if let Some(stream) = &mut self.stream {
            // Read lines until we receive the desired substring
            loop {
                let read_result = stream.recv().await;
                match read_result {
                    Ok(output) => {
                        let inbound_message = output;
                        if inbound_message.trim().is_empty() {
                            // Stream is completed - process is finished
                            gadget_logging::debug!(
                                "{} : STREAM COMPLETED - ENDING",
                                self.process_name.to_string_lossy()
                            );
                            return Ok(ProcessOutput::Exhausted(messages));
                        }
                        gadget_logging::debug!(
                            "{} : MESSAGE LOG : {}",
                            self.process_name.to_string_lossy(),
                            inbound_message.clone()
                        );
                        messages.push(inbound_message.clone());
                        if let Some(output) = &mut self.output {
                            output.send(inbound_message.clone()).unwrap();
                        }
                        if inbound_message.contains("nonexistent: command not found") {
                            return Err(Error::InvalidCommand("Command nonexistent".to_string()));
                        } else if inbound_message.contains(&substring) {
                            // We should now return with the output
                            return Ok(ProcessOutput::Output(messages));
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
        Err(Error::ReadError(
            "Failed to read output stream: None".to_string(),
        ))
    }

    pub(crate) fn status(&self) -> Status {
        let s = System::new();
        match s.process(self.pid.unwrap_or(Pid::from(0))) {
            Some(p) => match p.status() {
                ProcessStatus::Run => Status::Active,
                ProcessStatus::Sleep => Status::Sleeping,
                ProcessStatus::Stop => Status::Stopped,
                _ => Status::Dead,
            },
            None => Status::Dead,
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

impl From<ProcessStatus> for Status {
    fn from(status: ProcessStatus) -> Self {
        match status {
            ProcessStatus::Run => Status::Active,
            ProcessStatus::Sleep => Status::Sleeping,
            ProcessStatus::Stop => Status::Stopped,
            _ => Status::Dead,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializedGadgetProcess {
    pub command: String,
    pub process_name: OsString,
    #[serde(skip)]
    pub pid: Option<Pid>,
    pub status: Status,
}

impl From<&GadgetProcess> for SerializedGadgetProcess {
    fn from(process: &GadgetProcess) -> Self {
        Self {
            command: process.command.clone(),
            process_name: process.process_name.clone(),
            pid: process.pid,
            status: process.status,
        }
    }
}

impl From<SerializedGadgetProcess> for GadgetProcess {
    fn from(process: SerializedGadgetProcess) -> Self {
        Self {
            command: process.command,
            process_name: process.process_name,
            pid: process.pid,
            stream: None,
            output: None,
            status: process.status,
        }
    }
}
