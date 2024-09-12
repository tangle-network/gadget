use crate::sdk::config::SingleGadgetConfig;
use crate::sdk::setup::{generate_node_input, SingleGadgetInput};
use futures::Future;
use gadget_io::{GadgetConfig, KeystoreConfig, SupportedChains};
use gadget_sdk::clients::tangle::runtime::TangleRuntimeClient;
use gadget_sdk::logger::Logger;
use gadget_sdk::store::KeyValueStoreBackend;
use structopt::StructOpt;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::ServiceBlueprint;
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

/// Runs a gadget for a given protocol.
pub async fn run_gadget_for_protocol<
    KBE: KeyValueStoreBackend,
    T: FnOnce(SingleGadgetInput<KBE>) -> F,
    F,
    T2: FnOnce() -> F2,
    F2: SendFuture<'static, KBE>,
>(
    _client: TangleRuntimeClient,
    services: Vec<ServiceBlueprint>,
    n_protocols: usize,
    keystore_backend: T2,
    executor: T,
) -> color_eyre::Result<()>
where
    F: SendFuture<'static, ()>,
{
    let args = std::env::args();
    println!("Args: {args:?}");
    let config = GadgetConfig::from_iter_safe(args);

    if config.is_err() {
        return Err(color_eyre::Report::msg(format!(
            "Failed to parse gadget config: {config:?}"
        )));
    }

    let config = config.unwrap();
    let keystore_backend = keystore_backend().await;
    let keystore =
        keystore_from_base_path(&config.base_path, config.chain, config.keystore_password);

    let logger = Logger {
        id: "run-gadget".to_string(),
    };

    logger.info("Starting gadget with config: {config:?}");

    let (node_input, network_handle) = generate_node_input(SingleGadgetConfig {
        keystore_backend,
        services,
        keystore,
        base_path: config.base_path,
        bind_ip: config.bind_addr,
        bind_port: config.bind_port,
        bootnodes: config.bootnodes,
        n_protocols,
    })
    .await?;

    let protocol_future = gadget_io::tokio::task::spawn(executor(node_input));

    gadget_io::tokio::select! {
        res0 = protocol_future => {
            Err(color_eyre::Report::msg(format!("Protocol future unexpectedly finished: {res0:?}")))
        },

        res1 = network_handle => {
            Err(color_eyre::Report::msg(format!("Networking future unexpectedly finished: {res1:?}")))
        },
    }
}

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
