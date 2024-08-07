use color_eyre::{eyre::eyre, Result};
use std::process::Command;

use super::CommandInstalled;

pub struct Forge {
    cmd: Command,
}

impl Forge {
    /// Creates a new Forge instance.
    pub fn new() -> Forge {
        let cmd = Command::new("forge");
        Self { cmd }
    }

    /// Returns the version of Forge.
    pub fn version(&mut self) -> Result<String> {
        let out = self.cmd.arg("--version").output()?.stdout;
        Ok(String::from_utf8(out)?.trim().replace("forge", ""))
    }

    /// Returns true if Forge is installed.
    pub fn is_installed(&self) -> bool {
        self.cmd.is_installed()
    }

    /// Builds the contracts.
    pub fn build(&mut self) -> Result<()> {
        eprintln!("Building contracts...");
        let status = self
            .cmd
            .arg("build")
            .stderr(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .spawn()?
            .wait()?;
        if !status.success() {
            return Err(eyre!("Failed to build contracts"));
        }
        Ok(())
    }
}
