use color_eyre::eyre::WrapErr;
use color_eyre::*;
use console_error_panic_hook;
use gadget_io::{into_js_error, log, KeystoreConfig, Opt, SupportedChains, TomlConfig};
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::panic;
use std::{fmt::Display, net::IpAddr, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures;

use crate::config;
use crate::shell;

use gadget_common::prelude::*;
use gadget_common::ExecutableJob;
use tsify::Tsify;

use futures::{select, FutureExt};
use futures_timer::Delay;
use matchbox_socket::{PeerState, WebRtcSocket};
use std::time::Duration;

// use wasm_bindgen_test::wasm_bindgen_test;
use log::info;

#[wasm_bindgen]
#[no_mangle]
pub async fn run_web_shell(
    config: TomlConfig,
    options: Opt,
    keys: Vec<String>,
) -> Result<JsValue, JsValue> {
    // web_init();
    // color_eyre::install().map_err(|e| js_sys::Error::new(&e.to_string()))?;
    // let opt = Opt::from_args();
    // setup_logger(&opt, "gadget_shell")?;
    // log(&log_rust(&format!("Hello from web_main in Rust!")));
    log(&format!("TomlConfig: {:?}", config));
    log(&format!("Options: {:?}", options));

    let TomlConfig {
        bind_ip,
        bind_port,
        url,
        bootnodes,
        node_key,
        base_path,
        keystore_password,
        chain,
    } = config;

    let Opt {
        config,
        verbose,
        pretty,
        options,
    } = options;

    let endpoint = Url::parse(&url).map_err(into_js_error)?;
    log(&format!("Endpoint: {:?}", endpoint));

    let bind_ip = IpAddr::from_str(&bind_ip).map_err(into_js_error)?;
    log(&format!("Bind IP: {:?}", bind_ip));

    let bootnodes: Vec<Multiaddr> = bootnodes
        .iter()
        .map(|s| Multiaddr::from_str(&s))
        .filter_map(|x| x.ok())
        .collect();
    log(&format!("Bootnodes: {:?}", bootnodes));

    let node_key: [u8; 32] = if let Some(node_key) = node_key {
        hex::decode(&node_key)
    } else {
        hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
        // TODO: Should generate
    }
    .map_err(into_js_error)?
    .as_slice()
    .try_into()
    .map_err(into_js_error)?;
    log(&format!("Node Key: {:?}", node_key));

    //let keys: [u8; 32] = hex::decode("0000000000000000000000000000000000000000000000000000000000000001").map_err(into_js_error)?.as_slice().try_into().map_err(into_js_error)?;

    // let node_key: [u8; 32] = hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
    //     .map_err(into_js_error)?
    //     .as_slice()
    //     .try_into()
    //     .map_err(into_js_error)?;

    shell::run_forever(config::ShellConfig {
        keystore: KeystoreConfig::InMemory { keystore: keys }, //"0000000000000000000000000000000000000000000000000000000000000001".to_string() },
        subxt: config::SubxtConfig { endpoint },
        base_path,
        bind_ip,
        bind_port,
        bootnodes,
        node_key,
    })
    .await
    .map_err(|e| js_sys::Error::new(&e.to_string()))?;
    let result = serde_wasm_bindgen::to_value("success message").map_err(into_js_error)?;
    Ok(result)
}
