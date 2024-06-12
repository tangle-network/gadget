use crate::process::manager::GadgetProcessManager;
use crate::process::types::GadgetInstructionData;
use crate::process::types::{CommandOrSequence, ProcessOutput};
use crate::process::utils::*;
use std::error::Error;

mod process;

// TODO: Gadget Setup following V2 - yet to be implemented

pub async fn run_executor(instructions: &str) {
    // TODO: New commands should be received real-time from connection - will follow new V2 structure
    let mut manager = GadgetProcessManager::new();
    let instructions: GadgetInstructionData =
        serde_json::from_str(instructions).expect("JSON was not well-formatted");

    // Execute all the required commands
    for to_execute in instructions.commands {
        let identifier = to_execute.name;
        let commands = to_execute.command;
        match commands {
            CommandOrSequence::Command(cmd) => {
                println!("Executing single command {:?}: {:?}", identifier, cmd);
                match manager.run(identifier, &cmd).await {
                    Ok(_identifier) => {
                        // Logs/Metrics
                    }
                    Err(_error) => {
                        // Error logging
                    }
                }
            }
            CommandOrSequence::Sequence(cmds) => {
                println!(
                    "Executing sequence of commands {:?}: {:?}",
                    identifier, cmds
                );
                // TODO: Utilizing the output of commands for new commands
            }
        }
    }

    // Receive input from all running processes
    let mut ended = Vec::new();
    for (service, process) in &mut manager.children {
        println!("LOG : Process {}", service.clone());
        let status = process.status().unwrap();
        println!("\tSTATUS: {:?}", status);
        let output = process.read().await;
        println!("\n{} read result:\n\t {:?}\n", service, output);
        if let ProcessOutput::Exhausted(_) = output {
            println!("{} ended, removing from Manager", service);
            ended.push(service.clone());
        }
    }
    manager.children.retain(|k, _| !ended.contains(k));

    println!(
        " ENDED EXECUTOR WITH GADGET PROCESS MANAGER:\n{:?}\n",
        manager
    );
}

pub async fn run_tangle_validator() -> Result<(), Box<dyn Error>> {
    let mut manager = GadgetProcessManager::new();

    // let install = manager.run(&*"binary_install", &*"wget https://github.com/webb-tools/tangle/releases/download/v1.0.0/tangle-default-linux-amd64".to_string()).await?;
    // let make_executable = manager.run(&*"make_executable", &*"chmod +x tangle-default-linux-amd64".to_string()).await?;

    // let start_validator = manager.run("tangle_validator".to_string(), "./tangle-default-linux-amd64").await?;
    let ping_test = manager
        .run("ping_test".to_string(), "ping -c 5 localhost")
        .await?;
    manager.focus_service_to_completion(ping_test).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor() {
        let the_file = r#"{
            "commands": [
                ["ping_google", "ping -c 5 google.com"],
                ["ping_local", "ping -c 5 localhost"],
                ["sequence_test", ["ping -c 5 localhost", "ls", "clear"]]
            ]
        }"#;
        run_executor(the_file).await;
    }

    #[tokio::test]
    async fn test_validator() {
        run_tangle_validator().await.unwrap();
    }
}
