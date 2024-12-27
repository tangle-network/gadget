use super::CommandInstalled;
use color_eyre::Result;
use gadget_logging::tracing;
use std::process::Command;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to build contracts")]
    BuildContracts,
    #[error("Failed to install dependencies")]
    InstallDependencies,
}

pub struct Forge {
    cmd: Command,
}

impl Default for Forge {
    fn default() -> Self {
        Self::new()
    }
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
    pub fn build(mut self) -> Result<()> {
        eprintln!("Building contracts...");
        let status = self
            .cmd
            .arg("build")
            .stderr(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .spawn()?
            .wait()?;
        if !status.success() {
            return Err(Error::BuildContracts.into());
        }
        Ok(())
    }

    pub fn install_dependencies(mut self) -> Result<()> {
        tracing::info!("Installing dependencies...");
        let output = self
            .cmd
            .args(["soldeer", "update", "-d"])
            .stderr(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .output()?;
        if !output.status.success() {
            return Err(Error::InstallDependencies.into());
        }
        Ok(())
    }
}
