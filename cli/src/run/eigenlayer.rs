use color_eyre::eyre::Result;
use std::path::PathBuf;
use std::process::Command;

pub struct RunOpts {
    pub http_rpc_url: String,
    pub aggregator_key: String,
    pub port: u16,
    pub task_manager: String,
    pub blueprint_path: PathBuf,
}

/// Run a compiled Eigenlayer AVS binary with the provided options
pub async fn run_eigenlayer_avs(opts: RunOpts) -> Result<()> {
    // First ensure the binary exists and is compiled
    let target_dir = opts.blueprint_path.join("target/release");
    let binary_name = opts
        .blueprint_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| color_eyre::eyre::eyre!("Invalid blueprint path"))?;

    // Check if binary exists, if not, build it
    let binary_path = target_dir.join(binary_name);
    if !binary_path.exists() {
        println!("Building AVS binary...");
        let status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&opts.blueprint_path)
            .status()?;

        if !status.success() {
            return Err(color_eyre::eyre::eyre!("Failed to build AVS binary"));
        }
    }

    // Run the AVS binary with the provided options
    println!("Starting AVS...");
    let child = Command::new(binary_path)
        .env("ETH_RPC_URL", opts.http_rpc_url)
        .env("AGGREGATOR_KEY", opts.aggregator_key)
        .env("PORT", opts.port.to_string())
        .env("TASK_MANAGER_ADDRESS", opts.task_manager)
        .current_dir(&opts.blueprint_path)
        .spawn()?;

    println!("AVS is running with PID: {}", child.id());
    Ok(())
}
