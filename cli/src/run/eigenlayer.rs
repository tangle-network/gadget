use color_eyre::eyre::{Result, eyre};
use gadget_config::{BlueprintEnvironment, supported_chains::SupportedChains};
use gadget_std::fs;
use gadget_std::path::PathBuf;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::{Child, Command};
use toml::Value;
use tracing::info;

fn get_binary_name() -> Result<String> {
    let cargo_toml = fs::read_to_string("Cargo.toml")?;
    let cargo_data: Value = toml::from_str(&cargo_toml)?;

    // First check for [[bin]] section
    if let Some(Value::Array(bins)) = cargo_data.get("bin") {
        if let Some(first_bin) = bins.first() {
            if let Some(name) = first_bin.get("name").and_then(|n| n.as_str()) {
                return Ok(name.to_string());
            }
        }
    }

    // If no [[bin]] section, try package name
    if let Some(package) = cargo_data.get("package") {
        if let Some(name) = package.get("name").and_then(|n| n.as_str()) {
            return Ok(name.to_string());
        }
    }

    Err(eyre!("Could not find binary name in Cargo.toml"))
}

/// Run a compiled Eigenlayer AVS binary with the provided options
pub async fn run_eigenlayer_avs(
    config: BlueprintEnvironment,
    chain: SupportedChains,
    binary_path: Option<PathBuf>,
) -> Result<Child> {
    let binary_path = if let Some(path) = binary_path {
        path
    } else {
        let target_dir = PathBuf::from("./target/release");
        let binary_name = get_binary_name()?;
        target_dir.join(&binary_name)
    };

    println!(
        "Attempting to run Eigenlayer AVS binary at: {}",
        binary_path.display()
    );

    // Build only if binary doesn't exist or no path was provided
    if !binary_path.exists() {
        info!("Binary not found at {}, building...", binary_path.display());
        let status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .status()
            .await?;

        if !status.success() {
            return Err(eyre!("Failed to build AVS binary"));
        }
    }

    // Get contract addresses
    let contract_addresses = config
        .protocol_settings
        .eigenlayer()
        .map_err(|_| eyre!("Missing Eigenlayer contract addresses"))?;

    // Run the AVS binary with the provided options
    println!("Starting AVS...");
    let mut command = Command::new(&binary_path);

    // Add the run subcommand
    command.arg("run");

    // Required arguments
    command
        .arg("--http-rpc-url")
        .arg(&config.http_rpc_endpoint)
        .arg("--ws-rpc-url")
        .arg(&config.ws_rpc_endpoint)
        .arg("--keystore-uri")
        .arg(&config.keystore_uri)
        .arg("--chain")
        .arg(chain.to_string())
        .arg("--protocol")
        .arg("eigenlayer");

    // Optional arguments
    // TODO: Implement Keystore Password
    // if let Some(password) = &config.keystore_password {
    //     command.arg("--keystore-password").arg(password);
    // }

    // Contract addresses
    command
        .arg("--registry-coordinator")
        .arg(contract_addresses.registry_coordinator_address.to_string())
        .arg("--operator-state-retriever")
        .arg(
            contract_addresses
                .operator_state_retriever_address
                .to_string(),
        )
        .arg("--delegation-manager")
        .arg(contract_addresses.delegation_manager_address.to_string())
        .arg("--strategy-manager")
        .arg(contract_addresses.strategy_manager_address.to_string())
        .arg("--service-manager")
        .arg(contract_addresses.service_manager_address.to_string())
        .arg("--stake-registry")
        .arg(contract_addresses.stake_registry_address.to_string())
        .arg("--avs-directory")
        .arg(contract_addresses.avs_directory_address.to_string())
        .arg("--rewards-coordinator")
        .arg(contract_addresses.rewards_coordinator_address.to_string());

    assert!(binary_path.exists(), "Binary path does not exist");

    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // Handle stdout
    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let mut stdout_reader = BufReader::new(stdout).lines();

    // Handle stderr
    let stderr = child.stderr.take().expect("Failed to capture stderr");
    let mut stderr_reader = BufReader::new(stderr).lines();

    // Spawn tasks to handle stdout and stderr
    let stdout_task = tokio::spawn(async move {
        while let Some(line) = stdout_reader
            .next_line()
            .await
            .expect("Failed to read stdout")
        {
            println!("{}", line);
        }
    });

    let stderr_task = tokio::spawn(async move {
        while let Some(line) = stderr_reader
            .next_line()
            .await
            .expect("Failed to read stderr")
        {
            eprintln!("{}", line);
        }
    });

    // Wait for both tasks to complete
    let _ = tokio::join!(stdout_task, stderr_task);

    println!(
        "AVS is running with PID: {}",
        child.id().unwrap_or_default()
    );
    Ok(child)
}
