use super::error::Error;
use crate::executor::process::types::{GadgetProcess, ProcessOutput, Status};
use crate::executor::process::utils::*;
use crate::executor::OS_COMMAND;
use crate::{craft_child_process, run_command};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::System;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;

/// Manager for gadget-executor process. The processes are recorded to be controlled by their Service name.
/// This Manager can be reconstructed from a file to recover a gadget-executor.
#[derive(Serialize, Deserialize, Debug)]
pub struct GadgetProcessManager {
    /// Hashmap that contains all the children spawned by this Manager. Keys are the names of each Service.
    pub children: HashMap<String, GadgetProcess>,
}

impl GadgetProcessManager {
    pub fn new() -> GadgetProcessManager {
        GadgetProcessManager {
            children: HashMap::new(),
        }
    }

    /// Load the state of previously running processes to recover gadget-executor
    #[allow(dead_code)]
    pub(crate) async fn new_from_saved(
        file: &str,
    ) -> Result<GadgetProcessManager, crate::error::Error> {
        let file = std::fs::File::open(file)?;
        let mut new_manager: GadgetProcessManager = serde_json::from_reader(file)?;

        // Restarts processes that were previously running
        new_manager.restart_dead().await?;

        Ok(new_manager)
    }

    /// Store the state of the current processes
    #[allow(dead_code)]
    pub(crate) async fn save_state(&self) -> Result<String, crate::error::Error> {
        let serialized_data = serde_json::to_string(self)?;
        let mut file = File::create("./savestate.json").await?;
        file.write_all(serialized_data.clone().as_bytes()).await?;
        Ok(serialized_data)
    }

    /// Runs the given command and stores it using the identifier as the key. Returns the identifier used
    #[allow(unused_results)]
    pub async fn run(&mut self, identifier: String, command: &str) -> Result<String, Error> {
        let gadget_process = run_command!(command)?;
        self.children.insert(identifier.clone(), gadget_process);
        Ok(identifier)
    }

    /// Runs the given command and returns a [stream](broadcast::Receiver) of its output
    #[allow(unused_results)]
    pub async fn start_process_and_get_output(
        &mut self,
        identifier: String,
        command: &str,
    ) -> Result<broadcast::Receiver<String>, Error> {
        self.run(identifier.clone(), command).await?;
        let process = self
            .children
            .get_mut(&identifier)
            .ok_or(Error::ServiceNotFound(identifier))?;
        process.resubscribe()
    }

    /// Focuses on the given service until its stream is exhausted, meaning that the process ran to completion.
    pub async fn focus_service_to_completion(&mut self, service: String) -> Result<String, Error> {
        let process = self
            .children
            .get_mut(&service)
            .ok_or(Error::ServiceNotFound(service))?;
        let mut output_stream = String::new();
        loop {
            match process.read_until_default_timeout().await {
                ProcessOutput::Output(output) => {
                    output_stream.push_str(&format!("{output:?}\n"));
                    continue;
                }
                ProcessOutput::Exhausted(output) => {
                    output_stream.push_str(&format!("{output:?}\n"));
                    break;
                }
                ProcessOutput::Waiting => {
                    continue;
                }
            }
        }
        Ok(output_stream)
    }

    /// Focuses on the given service until its output includes the substring provided. Returns a
    /// ProcessOutput. ProcessOutput::Output means that the substring was received,
    /// ProcessOutput::Exhausted means that the substring was not found and the stream was
    /// exhausted, ProcessOutput::Waiting means that the substring was never found and the stream
    /// is not exhausted (an error likely occurred).
    pub(crate) async fn focus_service_until_output_contains(
        &mut self,
        service: String,
        specified_output: String,
    ) -> Result<ProcessOutput, Error> {
        let process = self
            .children
            .get_mut(&service)
            .ok_or(Error::ServiceNotFound(service))?;
        Ok(process.read_until_receiving_string(specified_output).await)
    }

    /// Removes processes that are no longer running from the manager. Returns a Vector of the names of processes removed
    #[allow(dead_code)]
    pub(crate) async fn remove_dead(&mut self) -> Result<Vec<String>, Error> {
        let dead_processes = Vec::new();
        let mut to_remove = Vec::new();
        let s = System::new_all();

        // Find dead processes and gather them for return & removal
        for (key, value) in self.children.iter() {
            let current_pid = value.pid;
            if let Some(process) = s.process(current_pid) {
                if process.name() == value.process_name {
                    // Still running
                    continue;
                } else {
                    // No longer running, some unknown process is now utilizing this PID
                    to_remove.push(key.clone());
                }
            }
        }
        self.children.retain(|k, _| !to_remove.contains(k));

        // TODO: If dead children are `supposed` to be running, we should start them up again instead of just removing them

        Ok(dead_processes)
    }

    /// Finds all dead processes that still exist in map and starts them again. This function
    /// is used to restart all processes after loading a Manager from a file.
    #[allow(unused_results)]
    pub(crate) async fn restart_dead(&mut self) -> Result<(), Error> {
        let mut restarted_processes = Vec::new();
        let mut to_remove = Vec::new();
        // Find dead processes and restart them
        for (key, value) in self.children.iter_mut() {
            match value.status()? {
                Status::Active | Status::Sleeping => {
                    // TODO: Metrics + Logs for these living processes
                    // Check if this process is still running what is expected
                    // If it is still correctly running, we just move along
                    continue;
                }
                Status::Inactive | Status::Dead => {
                    // Dead, should be restarted
                }
                Status::Unknown(code) => {
                    println!("LOG : {} yielded {}", key.clone(), code);
                }
            }
            restarted_processes.push((key.clone(), value.restart_process().await?));
            to_remove.push(key.clone());
        }
        self.children.retain(|k, _| !to_remove.contains(k));
        for (service, restarted) in restarted_processes {
            self.children.insert(service.clone(), restarted);
        }

        Ok(())
    }
}

impl Default for GadgetProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
