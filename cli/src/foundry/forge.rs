use super::CommandInstalled;
use std::process::Command;

pub struct Forge;

impl Default for Forge {
    fn default() -> Self {
        Self::new()
    }
}

impl Forge {
    /// Creates a new Forge instance.
    pub fn new() -> Forge {
        Forge
    }

    /// Returns the version of Forge.
    pub fn version(&self) -> color_eyre::Result<String> {
        let output = Command::new("forge").arg("--version").output()?;

        Ok(String::from_utf8(output.stdout)?
            .trim()
            .replace("forge", ""))
    }

    /// Returns true if Forge is installed.
    pub fn is_installed(&self) -> bool {
        Command::new("forge").is_installed()
    }

    pub fn install_dependencies(&self) -> color_eyre::Result<()> {
        println!("Installing dependencies...");
        let output = Command::new("forge")
            .args(["soldeer", "update", "-d"])
            .output()?;

        if !output.status.success() {
            return Err(color_eyre::eyre::eyre!(
                "Failed to install dependencies: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }

    pub fn build(&self) -> color_eyre::Result<()> {
        println!("Building contracts...");
        let output = Command::new("forge").arg("build").output()?;

        if !output.status.success() {
            return Err(color_eyre::eyre::eyre!(
                "Failed to build contracts: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }
}
