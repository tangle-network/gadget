use color_eyre::Result;
use std::path::PathBuf;
use structopt::StructOpt;
use shell_sdk::config::defaults;

pub mod protocols;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Shell Manager",
    about = "An MPC executor that connects to the Tangle network and runs protocols dynamically on the fly"
)]
struct Opt {
    /// The path to the configuration file. If not provided, the default configuration will be used.
    /// Note that if the configuration file is provided, the command line arguments will be ignored.
    #[structopt(global = true, parse(from_os_str), short = "c", long = "config")]
    config: Option<PathBuf>,
    /// The verbosity level, can be used multiple times
    #[structopt(long, short = "v", global = true, parse(from_occurrences))]
    verbose: i32,
    /// Whether to use pretty logging
    #[structopt(global = true, long)]
    pretty: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let opt = Opt::from_args();
    setup_logger(&opt, "gadget_shell")?;
    let config = if let Some(config) = opt.config {
        let config_contents = std::fs::read_to_string(config)?;
        toml::from_str(&config_contents)?
    } else {
        opt.options
    };
    shell_sdk::run_forever(shell_sdk::ShellConfig {
        keystore: shell_sdk::KeystoreConfig::Path {
            path: config
                .base_path
                .join("chains")
                .join(config.chain.to_string())
                .join("keystore"),
            password: config.keystore_password.map(|s| s.into()),
        },
        subxt: shell_sdk::SubxtConfig {
            endpoint: config.url,
        },
        base_path: config.base_path,
        bind_ip: config.bind_ip,
        bind_port: config.bind_port,
        bootnodes: config.bootnodes,
        node_key: hex::decode(
            config
                .node_key
                .unwrap_or_else(|| hex::encode(defaults::generate_node_key())),
        )?
        .try_into()
        .map_err(|_| {
            color_eyre::eyre::eyre!("Invalid node key length, expect 32 bytes hex string")
        })?,
    })
    .await?;
    Ok(())
}

/// Sets up the logger for the shell-sdk, based on the verbosity level passed in.
fn setup_logger(opt: &Opt, filter: &str) -> Result<()> {
    use tracing::Level;
    let log_level = match opt.verbose {
        0 => Level::ERROR,
        1 => Level::WARN,
        2 => Level::INFO,
        3 => Level::DEBUG,
        _ => Level::TRACE,
    };
    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(format!("{filter}={log_level}").parse()?)
        .add_directive(format!("gadget={log_level}").parse()?);
    let logger = tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_line_number(false)
        .without_time()
        .with_max_level(log_level)
        .with_env_filter(env_filter);
    if opt.pretty {
        logger.pretty().init();
    } else {
        logger.compact().init();
    }
    Ok(())
}
