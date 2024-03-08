use color_eyre::Result;

mod config;
mod keystore;
mod shell;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    setup_logger(3, "gadget_shell")?;
    // Hardcoded paths for now.
    let keystore_path = std::path::PathBuf::from(format!(
        "{}../../target/tangle/chains/local_testnet/keystore",
        env!("CARGO_MANIFEST_DIR")
    ));
    shell::run_forever(config::ShellConfig {
        keystore: config::KeystoreConfig::Path {
            path: keystore_path,
            password: None,
        },
    })
    .await?;
    Ok(())
}

/// Sets up the logger for the shell, based on the verbosity level passed in.
///
/// Returns `Ok(())` on success, or `Err(anyhow::Error)` on failure.
///
/// # Arguments
///
/// * `verbosity` - An i32 integer representing the verbosity level.
/// * `filter` -  An &str representing filtering directive for EnvFilter
pub fn setup_logger(verbosity: i32, filter: &str) -> Result<()> {
    use tracing::Level;
    let log_level = match verbosity {
        0 => Level::ERROR,
        1 => Level::WARN,
        2 => Level::INFO,
        3 => Level::DEBUG,
        _ => Level::TRACE,
    };
    let directive_1 = format!("{filter}={log_level}")
        .parse()
        .expect("valid log level");
    let directive_2 = format!("gadget={log_level}")
        .parse()
        .expect("valid log level");
    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(directive_1)
        .add_directive(directive_2);
    let logger = tracing_subscriber::fmt()
        .with_target(true)
        .with_max_level(log_level)
        .with_env_filter(env_filter);
    let logger = logger.pretty();

    logger.init();
    Ok(())
}
