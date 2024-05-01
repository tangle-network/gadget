use color_eyre::eyre::WrapErr;
use color_eyre::*;
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, net::IpAddr, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use url::Url;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures;
use console_error_panic_hook;
use std::panic;
use gadget_io::into_js_error as into_js_error;
use gadget_io::KeystoreConfig;

// use gadget_io::file_test;
use gadget_common::prelude::*;
use gadget_common::ExecutableJob;
use tsify::Tsify;

use futures::{select, FutureExt};
use futures_timer::Delay;
use matchbox_socket::{PeerState, WebRtcSocket};
use std::time::Duration;

// use wasm_bindgen_test::wasm_bindgen_test;
use log::info;

mod network;
mod config;
// mod keystore;
mod shell;
// mod tangle;
//
// #[derive(Debug, StructOpt)]
// #[structopt(
// name = "Gadget",
// about = "An MPC executor that connects to the Tangle network to perform work"
// )]
#[derive(Serialize, Deserialize, Debug, Tsify)]
#[tsify(from_wasm_abi)]
struct Opt {
    // /// The path to the configuration file. If not provided, the default configuration will be used.
    // /// Note that if the configuration file is provided, the command line arguments will be ignored.
    // #[structopt(global = true, parse(from_os_str), short = "c", long = "config")]
    config: Option<PathBuf>,
    // /// The verbosity level, can be used multiple times
    // #[structopt(long, short = "v", global = true, parse(from_occurrences))]
    verbose: i32,
    // /// Whether to use pretty logging
    // #[structopt(global = true, long)]
    pretty: bool,
    // /// The options for the shell
    // #[structopt(flatten)]
    options: TomlConfig,
}

// #[structopt(rename_all = "snake_case")]
#[derive(Default, Debug, StructOpt, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "snake_case")]
#[tsify(from_wasm_abi)]
enum SupportedChains {
    #[default]
    LocalTestnet,
    LocalMainnet,
    Testnet,
    Mainnet,
}

#[derive(Debug, StructOpt, Serialize, Deserialize, Tsify)]
#[tsify(from_wasm_abi)]
struct TomlConfig {
    // /// The IP address to bind to for the libp2p node.
    // #[structopt(long = "bind-ip", short = "i", default_value = defaults::BIND_IP)]
    // #[serde(default = "defaults::bind_ip")]
    bind_ip: String,
    // /// The port to bind to for the libp2p node.
    // #[structopt(long = "port", short = "p", default_value = defaults::BIND_PORT)]
    #[serde(default = "defaults::bind_port")]
    bind_port: u16,
    // /// The RPC URL of the Tangle Node.
    // #[structopt(long = "url", parse(try_from_str = url::Url::parse), default_value = defaults::RPC_URL)]
    // #[serde(default = "defaults::rpc_url")]
    url: String,
    // /// The List of bootnodes to connect to
    // #[structopt(long = "bootnodes", parse(try_from_str = <Multiaddr as std::str::FromStr>::from_str))]
    // #[serde(default)]
    bootnodes: Vec<String>,
    // /// The node key in hex format. If not provided, a random node key will be generated.
    // #[structopt(long = "node-key", env, parse(try_from_str = parse_node_key))]
    // #[serde(skip_serializing)]
    node_key: Option<String>,
    // /// The base path to store the shell data, and read data from the keystore.
    // #[structopt(
    // parse(from_os_str),
    // long,
    // short = "d",
    // required_unless = "config",
    // default_value_if("config", None, ".")
    // )]
    base_path: PathBuf,
    // /// Keystore Password, if not provided, the password will be read from the environment variable.
    // #[structopt(long = "keystore-password", env)]
    keystore_password: Option<String>,
    // /// The chain to connect to, must be one of the supported chains.
    // #[structopt(
    // long,
    // default_value,
    // possible_values = &[
    // "local_testnet",
    // "local_mainnet",
    // "testnet",
    // "mainnet"
    // ]
    // )]
    #[serde(default)]
    chain: SupportedChains,
}

// #[derive(Serialize, Deserialize, Debug, Tsify)]
// #[tsify(from_wasm_abi)]
// struct WebKeys {
//     // /// The path to the configuration file. If not provided, the default configuration will be used.
//     // /// Note that if the configuration file is provided, the command line arguments will be ignored.
//     // #[structopt(global = true, parse(from_os_str), short = "c", long = "config")]
//     config: Option<PathBuf>,
//     // /// The verbosity level, can be used multiple times
//     // #[structopt(long, short = "v", global = true, parse(from_occurrences))]
//     verbose: i32,
//     // /// Whether to use pretty logging
//     // #[structopt(global = true, long)]
//     pretty: bool,
//     // /// The options for the shell
//     // #[structopt(flatten)]
//     options: TomlConfig,
// }

// #[gadget_io::tokio::main(flavor = "current_thread")]
// async fn main() -> Result<()> {
// color_eyre::install()?;
// let opt = Opt::from_args();
// setup_logger(&opt, "gadget_shell")?;
// let config = if let Some(config) = opt.config {
//     let config_contents = std::fs::read_to_string(config)?;
//     toml::from_str(&config_contents)?
// } else {
//     opt.options
// };
// shell::run_forever(config::ShellConfig {
//     keystore: config::KeystoreConfig::Path {
//         path: config
//             .base_path
//             .join("chains")
//             .join(config.chain.to_string())
//             .join("keystore"),
//         password: config.keystore_password.map(|s| s.into()),
//     },
//     subxt: config::SubxtConfig {
//         endpoint: config.url,
//     },
//     base_path: config.base_path,
//     bind_ip: config.bind_ip,
//     bind_port: config.bind_port,
//     bootnodes: config.bootnodes,
//     node_key: hex::decode(
//         config
//             .node_key
//             .unwrap_or_else(|| hex::encode(defaults::generate_node_key())),
//     )?
//         .try_into()
//         .map_err(|_| {
//             color_eyre::eyre::eyre!("Invalid node key length, expect 32 bytes hex string")
//         })?,
// })
//     .await?;
//     Ok(())
// }

fn web_init() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
#[no_mangle]
pub async fn web_main(config: TomlConfig, options: Opt, keys: Vec<String>) -> Result<JsValue, JsValue> {
    web_init();
    color_eyre::install().map_err(|e| js_sys::Error::new(&e.to_string()))?;
    // let opt = Opt::from_args();
    // setup_logger(&opt, "gadget_shell")?;
    log(&log_rust(&format!("Hello from web_main in Rust!")));
    log(&format!("TomlConfig: {:?}", config));
    log(&format!("Options: {:?}", options));

    // let config = if let Some(config) = opt.config {
    //     let config_contents = std::fs::read_to_string(config)?;
    //     toml::from_str(&config_contents)?
    // } else {
    //     opt.options
    // };
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
        // .map_err(into_js_error)?
        // .as_slice()
        // .try_into()
        // .map_err(into_js_error)?;
    } else {
        hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
        // .map_err(into_js_error)?
        // .as_slice()
        // .try_into()
        // .map_err(into_js_error)?;
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
        keystore: KeystoreConfig::InMemory{ keystore: keys }, //"0000000000000000000000000000000000000000000000000000000000000001".to_string() },
        subxt: config::SubxtConfig {
            endpoint,
        },
        base_path,
        bind_ip,
        bind_port,
        bootnodes,
        node_key,
    })
        .await
    //let result = serde_wasm_bindgen::to_value("success message").map_err(into_js_error)?;
    //Ok(result)
}

// /// Sets up the logger for the shell, based on the verbosity level passed in.
// #[wasm_bindgen]
// pub fn setup_logger(opt: &Opt, filter: &str) -> Result<()> {
//     use tracing::Level;
//     let log_level = match opt.verbose {
//         0 => Level::ERROR,
//         1 => Level::WARN,
//         2 => Level::INFO,
//         3 => Level::DEBUG,
//         _ => Level::TRACE,
//     };
//     let env_filter = tracing_subscriber::EnvFilter::from_default_env()
//         .add_directive(format!("{filter}={log_level}").parse()?)
//         .add_directive(format!("gadget={log_level}").parse()?);
//     let logger = tracing_subscriber::fmt()
//         .with_target(false)
//         .with_level(true)
//         .with_line_number(false)
//         .without_time()
//         .with_max_level(log_level)
//         .with_env_filter(env_filter);
//     if opt.pretty {
//         logger.pretty().init();
//     } else {
//         logger.compact().init();
//     }
//     Ok(())
// }

mod defaults {
    pub const BIND_PORT: &str = "30555";
    pub const BIND_IP: &str = "0.0.0.0";
    pub const RPC_URL: &str = "ws://127.0.0.1:9944";

    pub fn rpc_url() -> url::Url {
        url::Url::parse(RPC_URL).expect("Default RPC URL is valid")
    }

    pub fn bind_ip() -> std::net::IpAddr {
        BIND_IP.parse().expect("Default bind IP is valid")
    }

    pub fn bind_port() -> u16 {
        BIND_PORT.parse().expect("Default bind port is valid")
    }

    // /// Generates a random node key
    // pub fn generate_node_key() -> [u8; 32] {
    //     use rand::Rng;
    //
    //     let mut rng = rand::thread_rng();
    //     let mut array = [0u8; 32];
    //     rng.fill(&mut array);
    //     array
    // }
}

// fn parse_node_key(s: &str) -> Result<String> {
//     let result: [u8; 32] = hex::decode(s.replace("0x", ""))?.try_into().map_err(|_| {
//         color_eyre::eyre::eyre!("Invalid node key length, expect 32 bytes hex string")
//     })?;
//     Ok(hex::encode(result))
// }

impl FromStr for SupportedChains {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "local_testnet" => Ok(SupportedChains::LocalTestnet),
            "local_mainnet" => Ok(SupportedChains::LocalMainnet),
            "testnet" => Ok(SupportedChains::Testnet),
            "mainnet" => Ok(SupportedChains::Mainnet),
            _ => Err(format!("Invalid chain: {}", s)),
        }
    }
}

impl Display for SupportedChains {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupportedChains::LocalTestnet => write!(f, "local_testnet"),
            SupportedChains::LocalMainnet => write!(f, "local_mainnet"),
            SupportedChains::Testnet => write!(f, "testnet"),
            SupportedChains::Mainnet => write!(f, "mainnet"),
        }
    }
}

#[wasm_bindgen(module = "/js-utils.js")]
extern "C" {
    fn webPrompt(message: &str) -> String;

    fn setConsoleInput();

    fn log_rust(s: &str) -> String;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// async fn get_from_js() -> Result<JsValue, JsValue> {
//     let promise = getUserFile();//js_sys::Promise::resolve(&42.into());
//     let result = wasm_bindgen_futures::JsFuture::from(promise).await?;
//     Ok(result)
// }

// #[wasm_bindgen(module = "/async-js-utils.js")]
// extern "C" {
//     #[wasm_bindgen(catch)]
//     async fn getUserFile() -> Result<JsValue, JsValue>;
// }

// #[wasm_bindgen(start)]
// fn run() {
//     log(&format!("Hello from {}!", name())); // should output "Hello from Rust!"
//
//     let x = MyClass::new();
//     assert_eq!(x.number(), 42);
//     x.set_number(10);
//     log(&x.render());
// }
//
#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    use wasm_bindgen_test::wasm_bindgen_test;
    wasm_bindgen_test_configure!(run_in_browser);

// #[wasm_bindgen_test]
// async fn test_main() -> Result<()> {
// setConsoleInput();
// file_test().await;
//
// color_eyre::install().expect("Failed to install color_eyre");
// let opt = Opt::from_args();


// let opt = Opt {
//     config: None,
//     verbose: 0,
//     pretty: false,
//     options: TomlConfig {
//         bind_ip: defaults::bind_ip(),
//         bind_port: 30555,
//         url: defaults::rpc_url(),
//         bootnodes: [].to_vec(),
//         node_key: Some("0000000000000000000000000000000000000000000000000000000000000000".to_string()),
//         base_path: "../tangle/tmp/alice".into(),
//         keystore_password: None,
//         chain: SupportedChains::LocalTestnet,
//     },
// };
// setup_logger(&opt, "gadget_web_shell").unwrap();
// let config = if let Some(config) = opt.config {
//     let config_contents = std::fs::read_to_string(config).unwrap();
//     toml::from_str(&config_contents).unwrap()
// } else {
//     opt.options
// };
// log(&format!("Resulting config: {:?}.", config));
// shell::run_forever(config::ShellConfig {
//     keystore: config::KeystoreConfig::InMemory,
//     subxt: config::SubxtConfig {
//         endpoint: config.url,
//     },
//     base_path: config.base_path,
//     bind_ip: config.bind_ip,
//     bind_port: config.bind_port,
//     bootnodes: config.bootnodes,
//     node_key: hex::decode(
//         config
//             .node_key
//             .unwrap_or_else(|| hex::encode(defaults::generate_node_key())),
//     )?
//         .try_into()
//         .map_err(|_| {
//             color_eyre::eyre::eyre!("Invalid node key length, expect 32 bytes hex string")
//         })?,
// })
//     .await.unwrap();
//     Ok(())
// }

// #[wasm_bindgen_test]
// pub fn color_eyre_simple() {
//     use color_eyre::eyre::WrapErr;
//     use color_eyre::*;
//
//     install().expect("Failed to install color_eyre");
//     let err_str = format!(
//         "{:?}",
//         Err::<(), Report>(eyre::eyre!("Base Error"))
//             .note("A note")
//             .suggestion("A suggestion")
//             .wrap_err("A wrapped error")
//             .unwrap_err()
//     );
//     // Print it out so if people run with `-- --nocapture`, they
//     // can see the full message.
//     println!("Error String is:\n\n{}", err_str);
//     assert!(err_str.contains("A wrapped error"));
//     assert!(err_str.contains("A suggestion"));
//     assert!(err_str.contains("A note"));
//     assert!(err_str.contains("Base Error"));
// }

/// Tests Browser-to-Browser connection using Matchbox WebRTC. Requires example matchbox_server
/// running as signal server
#[wasm_bindgen_test]
fn test_matchbox_browser_to_browser() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();

    let peer_one = async move {
        log(&format!("Peer One Beginning"));
        let (mut socket, loop_fut) = matchbox_socket_tester().await;
        let loop_fut = loop_fut.fuse();
        futures::pin_mut!(loop_fut);
        let timeout = Delay::new(Duration::from_millis(100));
        futures::pin_mut!(timeout);
        // Listening Loop
        loop {
            for (peer, state) in socket.update_peers() {
                match state {
                    PeerState::Connected => {
                        log(&format!("Peer joined: {peer}"));
                        let packet = "Message for Peer Two...".as_bytes().to_vec().into_boxed_slice();
                        socket.send(packet, peer);
                    }
                    PeerState::Disconnected => {
                        log(&format!("Peer {peer} Disconnected, Closing Socket"));
                        socket.close();
                    }
                }
            }
            for (peer, packet) in socket.receive() {
                let message = String::from_utf8_lossy(&packet);
                log(&format!("Message from {peer}: {message}"));
            }
            select! {
                // Loop every 100ms
                _ = (&mut timeout).fuse() => {
                    timeout.reset(Duration::from_millis(100));
                }
                // Break if the message loop ends via disconnect/closure
                _ = &mut loop_fut => {
                    break;
                }
            }
        }
    };

    let peer_two = async move {
        log(&format!("Peer Two Beginning"));
        let (mut socket, loop_fut) = matchbox_socket_tester().await;
        let loop_fut = loop_fut.fuse();
        futures::pin_mut!(loop_fut);
        let timeout = Delay::new(Duration::from_millis(100));
        futures::pin_mut!(timeout);
        // Listening Loop
        loop {
            for (peer, state) in socket.update_peers() {
                match state {
                    _ => continue
                }
            }
            for (peer, packet) in socket.receive() {
                let message = String::from_utf8_lossy(&packet);
                log(&format!("Peer Two Received Message from {peer}: {message}"));
                log(&format!("Peer Two Closing Socket"));
                socket.close();
            }

            select! {
                // Loop every 100ms
                _ = (&mut timeout).fuse() => {
                    timeout.reset(Duration::from_millis(100));
                }
                // Break if the message loop ends via disconnect/closure
                _ = &mut loop_fut => {
                    break;
                }
            }
        }
    };

    wasm_bindgen_futures::spawn_local(peer_one);
    wasm_bindgen_futures::spawn_local(peer_two);
}

async fn matchbox_socket_tester() -> (WebRtcSocket<SingleChannel>, MessageLoopFuture) {
    log(&format!("Starting Matchbox Listener"));
    let (mut socket, loop_fut) = WebRtcSocket::new_reliable("ws://localhost:3536/"); // Signaling Server Address
    (socket, loop_fut)
}

// async fn async_main() {
//     log(&format!("Hello from Matchbox Test!"));
//     let (mut socket, loop_fut) = network::setup::matchbox_listener().await;
//
// }

// #[wasm_bindgen_test]
// fn test_prompt() {
//     log(&format!("Input was {}!", webPrompt("Does this question appear?")));
// }

// #[wasm_bindgen_test]
// async fn test_file_input() {
//     log(&format!("File Success? {:?}!", get_from_js().await));
//     //log(&format!("Input was {}!", webPrompt("Does this question appear?")));
// }

}
