use crate::error::Error;
use crate::types::{GadgetProcess, ProcessOutput, SerializedGadgetProcess, Status};
use crate::utils::get_process_info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use sysinfo::{Pid, System};
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct GadgetProcessManager {
    pub children: HashMap<String, GadgetProcess>,
    pub system: System,
}

#[derive(Debug, Serialize, Deserialize)]
struct SerializedManager {
    children: HashMap<String, SerializedGadgetProcess>,
}

impl GadgetProcessManager {
    pub fn new() -> Self {
        Self {
            children: HashMap::new(),
            system: System::new_all(),
        }
    }

    pub async fn run(&mut self, id: String, command: &str) -> Result<String, Error> {
        let process = GadgetProcess::new(command.to_string()).await?;
        self.children.insert(id.clone(), process);
        Ok(id)
    }

    pub async fn run_with_output(
        &mut self,
        identifier: String,
        command: &str,
    ) -> Result<broadcast::Receiver<String>, Error> {
        self.run(identifier.clone(), command).await?;
        let process = self
            .children
            .get(&identifier)
            .ok_or_else(|| Error::ProcessNotFound(Pid::from(0)))?;
            
        process.get_output()
    }

    pub async fn remove_dead(&mut self) -> Vec<String> {
        let mut to_remove = Vec::new();
        
        self.refresh_processes();
        
        for (name, process) in &self.children {
            if let Some(pid) = process.pid {
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

    pub async fn restart_dead(&mut self) -> Result<(), Error> {
        let mut to_restart = Vec::new();
        
        for (name, process) in &self.children {
            match process.status {
                Status::Dead | Status::Stopped => {
                    to_restart.push((name.clone(), process.command.clone()));
                }
                _ => continue,
            }
        }
        
        for (name, command) in to_restart {
            self.run(name, &command).await?;
        }
        
        Ok(())
    }

    pub fn refresh_processes(&mut self) {
        // Refresh all system processes
        self.system.refresh_all();
        
        for process in self.children.values_mut() {
            if let Some(pid) = process.pid {
                if !self.system.processes().contains_key(&pid) {
                    process.status = Status::Dead;
                }
            }
        }
    }

    pub async fn save_state(&self) -> Result<(), Error> {
        let serialized: HashMap<String, SerializedGadgetProcess> = self
            .children
            .iter()
            .map(|(k, v)| (k.clone(), SerializedGadgetProcess::from(v)))
            .collect();

        let json = serde_json::to_string(&serialized)?;
        fs::write("./savestate.json", json)?;
        Ok(())
    }

    pub async fn load_state(path: &str) -> Result<Self, Error> {
        let json = fs::read_to_string(path)?;
        let serialized: HashMap<String, SerializedGadgetProcess> = serde_json::from_str(&json)?;
        
        let children: HashMap<String, GadgetProcess> = serialized
            .into_iter()
            .map(|(k, v)| (k, GadgetProcess::from(v)))
            .collect();

        Ok(Self {
            children,
            system: System::new_all(),
        })
    }

    pub async fn kill(&mut self, id: &str) -> Result<(), Error> {
        if let Some(process) = self.children.get_mut(id) {
            process.kill().await?;
            self.children.remove(id);
        }
        Ok(())
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

    pub async fn cleanup_service(&mut self, service: &str) -> Result<(), Error> {
        if let Some(process) = self.children.get_mut(service) {
            match process.pid {
                Some(pid) => {
                    if let Some(name) = get_process_info(pid.as_u32()) {
                        if name == process.process_name {
                            process.kill().await?;
                        }
                    }
                }
                None => {}
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
