use crate::shell::ShellNodeInput;
use crate::{defaults, ShellTomlConfig, SupportedChains};
use gadget_common::prelude::KeystoreBackend;
use gadget_core::job_manager::SendFuture;
use structopt::StructOpt;
use tangle_subxt::tangle_testnet_runtime::api::jobs::events::job_refunded::RoleType;
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

pub fn keystore_from_base_path(
    base_path: &std::path::Path,
    chain: SupportedChains,
    keystore_password: Option<String>,
) -> crate::KeystoreConfig {
    crate::KeystoreConfig::Path {
        path: base_path
            .join("chains")
            .join(chain.to_string())
            .join("keystore"),
        password: keystore_password.map(|s| s.into()),
    }
}

/// Runs a shell for a given protocol.
pub async fn run_shell_for_protocol<KBE: KeystoreBackend, T: FnOnce(ShellNodeInput<KBE>) -> F, F>(
    role_types: Vec<RoleType>,
    n_protocols: usize,
    keystore_backend: KBE,
    executor: T,
) -> color_eyre::Result<()>
where
    F: SendFuture<'static, ()>,
{
    // The args will be passed here by the shell-manager
    let args = std::env::args().into_iter().collect::<Vec<String>>();
    println!("Args: {args:?}");
    let config: ShellTomlConfig = ShellTomlConfig::from_iter(args);
    // setup_shell_logger(config.verbose, config.pretty, "gadget_shell")?;
    let keystore =
        keystore_from_base_path(&config.base_path, config.chain, config.keystore_password);

    let (node_input, network_handle) = crate::generate_node_input(crate::ShellConfig {
        keystore_backend,
        role_types,
        keystore,
        subxt: crate::SubxtConfig {
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
        n_protocols,
    })
    .await?;

    let protocol_future = tokio::task::spawn(executor(node_input));
    let ctrlc_future = tokio::signal::ctrl_c();

    tokio::select! {
        res0 = protocol_future => {
            Err(color_eyre::Report::msg(format!("Protocol future unexpectedly finished: {res0:?}")))
        },

        res1 = network_handle => {
            Err(color_eyre::Report::msg(format!("Networking future unexpectedly finished: {res1:?}")))
        },

        _ = ctrlc_future => {
            Ok(())
        }
    }
}

/// Sets up the logger for the shell-sdk, based on the verbosity level passed in.
pub fn setup_shell_logger(verbose: i32, pretty: bool, filter: &str) -> color_eyre::Result<()> {
    use tracing::Level;
    let log_level = match verbose {
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
    if pretty {
        logger.pretty().init();
    } else {
        logger.compact().init();
    }
    Ok(())
}
pub fn setup_log() {
    let _ = SubscriberBuilder::default()
        .with_env_filter(EnvFilter::from_default_env())
        .finish()
        .try_init();

    std::panic::set_hook(Box::new(|info| {
        log::error!(target: "gadget", "Panic occurred: {info:?}");
    }));
}
