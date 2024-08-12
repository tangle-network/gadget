//! Foundry Utilities.
use std::process::Command;

mod forge;

pub struct FoundryToolchain {
    pub forge: forge::Forge,
}

/// Trait for checking if a command is installed.
trait CommandInstalled {
    /// Returns true if the command is installed.
    fn is_installed(&self) -> bool;
}

impl CommandInstalled for Command {
    fn is_installed(&self) -> bool {
        let cmd = self.get_program();
        if cfg!(target_os = "windows") {
            Command::new("where")
                .arg(cmd)
                .output()
                .is_ok_and(|v| v.status.success())
        } else {
            Command::new("which")
                .arg(cmd)
                .output()
                .is_ok_and(|v| v.status.success())
        }
    }
}

impl Default for FoundryToolchain {
    fn default() -> Self {
        Self::new()
    }
}

impl FoundryToolchain {
    /// Creates a new FoundryToolchain instance.
    pub fn new() -> Self {
        Self {
            forge: forge::Forge::new(),
        }
    }
    pub fn check_installed_or_exit(&self) {
        fn foundry_installation_instructions() {
            eprintln!("Please install Foundry, follow https://getfoundry.sh/ for instructions.");
        }
        if !self.forge.is_installed() {
            eprintln!("Forge is not installed.");
            foundry_installation_instructions();
        }
        // Add more tools here.
    }
}
