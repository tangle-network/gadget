use color_eyre::Result;
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, net::IpAddr, path::PathBuf, str::FromStr};
use structopt::StructOpt;

use gadget_io::*;//{TomlConfig, SupportedChains, Opt};

#[cfg(target_family = "wasm")]
use wasm_bindgen::{
    JsValue,
    prelude::*,
};

// pub use gadget_shell::*;

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

    /// Generates a random node key
    pub fn generate_node_key() -> [u8; 32] {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let mut array = [0u8; 32];
        rng.fill(&mut array);
        array
    }
}

fn parse_node_key(s: &str) -> Result<String> {
    let result: [u8; 32] = hex::decode(s.replace("0x", ""))?.try_into().map_err(|_| {
        color_eyre::eyre::eyre!("Invalid node key length, expect 32 bytes hex string")
    })?;
    Ok(hex::encode(result))
}

#[cfg(target_family = "wasm")]
#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    use wasm_bindgen_test::wasm_bindgen_test;
    wasm_bindgen_test_configure!(run_in_browser);

    use futures::{select, FutureExt};
    use futures_timer::Delay;
    use matchbox_socket::{PeerState, WebRtcSocket, SingleChannel, MessageLoopFuture};
    use std::time::Duration;
    use gadget_io::log;

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
