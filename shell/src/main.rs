use color_eyre::Result;
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, net::IpAddr, path::PathBuf, str::FromStr};
use structopt::StructOpt;

mod config;
mod keystore;
mod network;
mod shell;
mod tangle;
#[cfg(target_family = "wasm")]
mod web;

use gadget_io::*; //{TomlConfig, SupportedChains, Opt};
#[cfg(target_family = "wasm")]
use wasm_bindgen::{prelude::*, JsValue};

#[cfg(target_family = "wasm")]
async fn main() -> Result<JsValue, JsValue> {
    color_eyre::install().map_err(|e| js_sys::Error::new(&e.to_string()))?;
    // setup_logger(&opt, "gadget_shell")?;
    // crate::web::run_web_shell()?;
    let result = serde_wasm_bindgen::to_value("success message").map_err(into_js_error)?;
    Ok(result)
}

#[gadget_io::tokio::main]
#[cfg(not(target_family = "wasm"))]
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
    shell::run_forever(config::ShellConfig {
        keystore: config::KeystoreConfig::Path {
            path: config
                .base_path
                .join("chains")
                .join(config.chain.to_string())
                .join("keystore"),
            password: config.keystore_password.map(|s| s.into()),
        },
        subxt: config::SubxtConfig {
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

/// Sets up the logger for the shell, based on the verbosity level passed in.
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
