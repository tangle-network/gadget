use super::CommandInstalled;
use color_eyre::eyre::Result;
use dialoguer::console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Read, Write},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
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
        // Start the command with inherited stdio to show output directly
        let status = Command::new("forge")
            .args(["soldeer", "update", "-d"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(color_eyre::eyre::eyre!(
                "Failed to install dependencies. Check the output above for details."
            ));
        }

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

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");

        // Create readers for both stdout and stderr
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        // Create channels for output communication
        let (tx, rx) = mpsc::channel();
        let tx_stderr = tx.clone();

        // Keep a buffer of recent output lines
        let mut output_buffer = VecDeque::with_capacity(100);

        // Spawn thread to read stdout
        thread::spawn(move || {
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    let _ = tx.send(line);
                }
            }
        });

        // Spawn thread to read stderr
        thread::spawn(move || {
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    let _ = tx_stderr.send(line);
                }
            }
        });

        // Update spinner and show output while command runs
        while child.try_wait()?.is_none() {
            spinner.tick();

            // Check for new output
            while let Ok(line) = rx.try_recv() {
                // Only add non-duplicate lines
                if !output_buffer.contains(&line) {
                    output_buffer.push_back(line);

                    // Keep only the last 100 lines
                    if output_buffer.len() > 100 {
                        output_buffer.pop_front();
                    }

                    // Print just the new line
                    spinner.suspend(|| {
                        println!("{}", style(&output_buffer[output_buffer.len() - 1]).dim());
                    });
                }
            }

            thread::sleep(std::time::Duration::from_millis(100));
        }

        let status = child.wait()?;

        if !status.success() {
            spinner.finish_with_message("❌ Failed to build contracts");
            return Err(color_eyre::eyre::eyre!(
                "Failed to build contracts. Check the output above for details."
            ));
        }

        spinner.finish_with_message("✨ Contracts built successfully!");
        Ok(())
    }
}
