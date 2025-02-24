use super::CommandInstalled;
use color_eyre::eyre::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};

pub struct Forge {
    /// Default template to use if none specified
    pub default_template: String,
    /// Default commit to use if none specified
    pub default_commit: String,
}

impl Default for Forge {
    fn default() -> Self {
        Self::new()
    }
}

impl Forge {
    /// Creates a new Forge instance with default settings.
    pub fn new() -> Self {
        Self {
            default_template: "foundry-rs/forge-template".to_string(),
            default_commit: "main".to_string(),
        }
    }

    /// Returns the version of Forge.
    pub fn version(&self) -> Result<String> {
        let output = Command::new("forge").arg("--version").output()?;
        Ok(String::from_utf8(output.stdout)?
            .trim()
            .replace("forge", ""))
    }

    /// Returns true if Forge is installed.
    pub fn is_installed(&self) -> bool {
        Command::new("forge").is_installed()
    }

    /// Install dependencies with live progress updates.
    /// Shows real-time output from forge and tracks progress.
    pub fn install_dependencies(&self) -> Result<()> {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈")
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );

        // Start the command with piped output so we can read it
        let mut child = Command::new("forge")
            .args(["soldeer", "update", "-d"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");

        // Create readers for both stdout and stderr
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        // Read output in a separate thread to prevent blocking
        let stdout_lines = std::thread::spawn(move || {
            stdout_reader
                .lines()
                .filter_map(|line| line.ok())
                .collect::<Vec<String>>()
        });

        let stderr_lines = std::thread::spawn(move || {
            stderr_reader
                .lines()
                .filter_map(|line| line.ok())
                .collect::<Vec<String>>()
        });

        // Update spinner while command runs
        while child.try_wait()?.is_none() {
            spinner.tick();
            spinner.set_message("Installing dependencies...");
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // Get the output from both streams
        let stdout_output = stdout_lines.join().unwrap();
        let stderr_output = stderr_lines.join().unwrap();

        let status = child.wait()?;

        if !status.success() {
            spinner.finish_with_message("Failed to install dependencies");
            return Err(color_eyre::eyre::eyre!(
                "Failed to install dependencies:\n{}",
                stderr_output.join("\n")
            ));
        }

        spinner.finish_with_message("Dependencies installed successfully!");
        Ok(())
    }

    /// Build the contracts with progress tracking.
    pub fn build(&self) -> Result<()> {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈")
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );

        spinner.set_message("Building contracts...");

        let mut child = Command::new("forge")
            .arg("build")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Similar pattern to install_dependencies for live output
        while child.try_wait()?.is_none() {
            spinner.tick();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let status = child.wait()?;

        if !status.success() {
            spinner.finish_with_message("Failed to build contracts");
            let stderr = child.stderr.take().expect("Failed to capture stderr");
            let stderr_reader = BufReader::new(stderr);
            let error_output: Vec<String> =
                stderr_reader.lines().filter_map(|line| line.ok()).collect();

            return Err(color_eyre::eyre::eyre!(
                "Failed to build contracts:\n{}",
                error_output.join("\n")
            ));
        }

        spinner.finish_with_message("Contracts built successfully!");
        Ok(())
    }
}
