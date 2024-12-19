use super::error::Error;
use crate::types::{GadgetProcess, ProcessOutput, Status, SerializedGadgetProcess};
use crate::run_command;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::{Pid, System};
use tokio::sync::{broadcast, oneshot};

/// Manager for gadget-executor process. The processes are recorded to be controlled by their Service name.
/// This Manager can be reconstructed from a file to recover a gadget-executor.
#[derive(Debug)]
pub struct GadgetProcessManager {
    /// Hashmap that contains all the children spawned by this Manager. Keys are the names of each Service.
    pub children: HashMap<String, GadgetProcess>,
    system: System,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializedManager {
    children: HashMap<String, SerializedGadgetProcess>,
}

impl GadgetProcessManager {
    #[must_use]
    pub fn new() -> GadgetProcessManager {
        GadgetProcessManager {
            children: HashMap::new(),
            system: System::new(),
        }
    }

    /// Load the state of previously running processes to recover gadget-executor
    pub async fn new_from_saved(file: &str) -> Result<GadgetProcessManager, Error> {
        let file_content = tokio::fs::read_to_string(file).await.map_err(|e| {
            Error::StateRecoveryError(format!("Failed to read state file: {}", e))
        })?;
        
        let serialized: SerializedManager = serde_json::from_str(&file_content).map_err(|e| {
            Error::StateRecoveryError(format!("Failed to parse state file: {}", e))
        })?;
        
        let mut new_manager = GadgetProcessManager {
            children: serialized.children.into_iter().map(|(k, v)| (k, GadgetProcess::from(v))).collect(),
            system: System::new(),
        };

        // Restarts processes that were previously running
        new_manager.restart_dead()?;
        Ok(new_manager)
    }

    /// Store the state of the current processes
    pub async fn save_state(&self) -> Result<(), Error> {
        let serialized = SerializedManager {
            children: self.children.iter().map(|(k, v)| (k.clone(), SerializedGadgetProcess::from(v.clone()))).collect(),
        };
        let json = serde_json::to_string(&serialized)?;
        tokio::fs::write("./savestate.json", json).await?;
        Ok(())
    }

    /// Runs the given command and stores it using the identifier as the key. Returns the identifier used
    ///
    /// # Errors
    /// - [`Error::UnexpectedExit`] - The execution of the command failed unexpectedly
    #[allow(unused_results)]
    pub async fn run(&mut self, identifier: String, command: &str) -> Result<String, Error> {
        let gadget_process = GadgetProcess::new(command.to_string()).await?;
        self.children.insert(identifier.clone(), gadget_process);
        Ok(identifier)
    }

    /// Runs the given command and returns a [stream](broadcast::Receiver) of its output
    ///
    /// # Errors
    /// - [`Error::UnexpectedExit`] - The execution of the command failed unexpectedly
    /// - [`Error::ServiceNotFound`] - The service did not exist when attempting to get its output
    #[allow(unused_results)]
    pub fn start_process_and_get_output(
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
    ///
    /// # Errors
    /// - [`Error::ServiceNotFound`] - The service did not exist when attempting to focus on it
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

    /// Focuses on the given service until its output includes the substring provided.
    ///
    /// # Returns
    /// - [`ProcessOutput`]
    ///     - [`ProcessOutput::Output`] - the substring was received
    ///     - [`ProcessOutput::Exhausted`] - the substring was not found and the stream was exhausted
    ///     - [`ProcessOutput::Waiting`] - the substring was never found and the stream is not exhausted (an error likely occurred).
    #[allow(dead_code)]
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
    pub(crate) async fn remove_dead(&mut self) -> Vec<String> {
        let mut to_remove = Vec::new();
        for (name, process) in self.children.iter() {
            if let Some(pid) = process.pid {
                self.system.refresh_process(pid);
                if !self.system.process(pid).is_some() {
                    to_remove.push(name.clone());
                }
            }
        }
        
        for name in &to_remove {
            self.children.remove(name);
        }
        
        to_remove
    }

    /// Finds all dead processes that still exist in map and starts them again. This function
    /// is used to restart all processes after loading a Manager from a file.
    ///
    /// # Errors
    /// -
    #[allow(unused_results)]
    pub(crate) async fn restart_dead(&mut self) -> Result<(), Error> {
        let mut restarted_processes = Vec::new();
        let mut to_remove = Vec::new();
        // Find dead processes and restart them
        for (key, value) in &mut self.children {
            match value.status() {
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
            restarted_processes.push((key.clone(), value.restart_process()?));
            to_remove.push(key.clone());
        }
        self.children.retain(|k, _| !to_remove.contains(k));
        for (service, restarted) in restarted_processes {
            self.children.insert(service.clone(), restarted);
        }

        Ok(())
    }

    /// Cleanup resources for a specific service
    async fn cleanup_service(&mut self, service: &str) -> Result<(), Error> {
        if let Some(process) = self.children.get(service) {
            if let Some(pid) = process.pid {
                match get_process_info(pid) {
                    Ok(Some(_)) => {
                        use nix::sys::signal::{self, Signal};
                        use nix::unistd::Pid as NixPid;
                        
                        signal::kill(NixPid::from_raw(pid.as_u32() as i32), Signal::SIGTERM)
                            .map_err(|e| Error::KillFailed(e, format!("Failed to terminate process {}", pid)))?;
                    }
                    Ok(None) => {
                        // Process no longer exists, just remove from our map
                    }
                    Err(e) => {
                        return Err(Error::StreamError(pid, e.to_string()));
                    }
                }
            }
        }
        self.children.remove(service);
        Ok(())
    }

    /// Cleanup all managed processes
    pub async fn cleanup_all(&mut self) -> Result<(), Error> {
        let services: Vec<String> = self.children.keys().cloned().collect();
        for service in services {
            if let Err(e) = self.cleanup_service(&service).await {
                log::error!("Failed to cleanup service {}: {}", service, e);
            }
        }
        Ok(())
    }
}

impl Default for GadgetProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
