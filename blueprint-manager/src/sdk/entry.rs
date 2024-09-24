use futures::Future;
use gadget_io::{KeystoreConfig, SupportedChains};
use tracing_subscriber::EnvFilter;

pub fn keystore_from_base_path(
    base_path: &std::path::Path,
    chain: SupportedChains,
    keystore_password: Option<String>,
) -> KeystoreConfig {
    KeystoreConfig::Path {
        path: base_path
            .join("chains")
            .join(chain.to_string())
            .join("keystore"),
        password: keystore_password.map(|s| s.into()),
    }
}

pub trait SendFuture<'a, T>: Send + Future<Output = T> + 'a {}
impl<'a, F: Send + Future<Output = T> + 'a, T> SendFuture<'a, T> for F {}

/// Sets up the logger for the blueprint manager, based on the verbosity level passed in.
pub fn setup_blueprint_manager_logger(
    verbose: i32,
    pretty: bool,
    filter: &str,
) -> color_eyre::Result<()> {
    use tracing::Level;
    let log_level = match verbose {
        0 => Level::ERROR,
        1 => Level::WARN,
        2 => Level::INFO,
        3 => Level::DEBUG,
        _ => Level::TRACE,
    };
    let env_filter =
        EnvFilter::from_default_env().add_directive(format!("{filter}={log_level}").parse()?);
    let logger = tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_line_number(false)
        .without_time()
        .with_max_level(log_level)
        .with_env_filter(env_filter);
    if pretty {
        let _ = logger.pretty().try_init();
    } else {
        let _ = logger.compact().try_init();
    }

    //let _ = env_logger::try_init();

    Ok(())
}
