// Original: https://github.com/paritytech/subxt/blob/3219659f12a36fe6b7408bf4ac1db184414c6c0c/testing/substrate-runner/src/lib.rs
#![allow(unused)]

use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Read};
use std::process::{self, Child, Command};
use std::sync::mpsc;
use std::thread;

/// Environment variable to set the path to the binary.
pub const TANGLE_NODE_ENV: &str = "TANGLE_NODE";

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    CouldNotExtractPort(String),
    CouldNotExtractP2pAddress(String),
    CouldNotExtractP2pPort(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err) => write!(f, "IO error: {err}"),
            Error::CouldNotExtractPort(log) => write!(
                f,
                "could not extract port from running substrate node's stdout: {log}"
            ),
            Error::CouldNotExtractP2pAddress(log) => write!(
                f,
                "could not extract p2p address from running substrate node's stdout: {log}"
            ),
            Error::CouldNotExtractP2pPort(log) => write!(
                f,
                "could not extract p2p port from running substrate node's stdout: {log}"
            ),
        }
    }
}

impl std::error::Error for Error {}

type CowStr = Cow<'static, str>;

#[derive(Debug, Clone)]
pub struct SubstrateNodeBuilder {
    binary_paths: Vec<String>,
    custom_flags: HashMap<CowStr, Option<CowStr>>,
}

impl Default for SubstrateNodeBuilder {
    fn default() -> Self {
        SubstrateNodeBuilder::new()
    }
}

impl SubstrateNodeBuilder {
    /// Configure a new Substrate node.
    pub fn new() -> Self {
        SubstrateNodeBuilder {
            binary_paths: vec![],
            custom_flags: Default::default(),
        }
    }

    /// Provide "tangle" as binary path.
    pub fn tangle(&mut self) -> &mut Self {
        self.binary_paths = vec!["tangle".into()];
        self
    }

    /// Set the path to the `substrate` binary; defaults to "substrate-node"
    /// or "substrate".
    pub fn binary_paths<I, S>(&mut self, paths: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.binary_paths = paths.into_iter().map(Into::into).collect();
        self
    }

    /// Add a single binary path.
    pub fn add_binary_path<S: Into<String>>(&mut self, path: S) -> &mut Self {
        self.binary_paths.push(path.into());
        self
    }

    /// Provide a boolean argument like `--alice`
    pub fn arg(&mut self, s: impl Into<CowStr>) -> &mut Self {
        self.custom_flags.insert(s.into(), None);
        self
    }

    /// Provide an argument with a value.
    pub fn arg_val(&mut self, key: impl Into<CowStr>, val: impl Into<CowStr>) -> &mut Self {
        self.custom_flags.insert(key.into(), Some(val.into()));
        self
    }

    /// Spawn the node, handing back an object which, when dropped, will stop it.
    pub fn spawn(mut self) -> Result<SubstrateNode, Error> {
        // Try to spawn the binary at each path, returning the
        // first "ok" or last error that we encountered.
        let mut res = Err(io::Error::new(
            io::ErrorKind::Other,
            "No binary path provided",
        ));

        let path = Command::new("mktemp")
            .arg("-d")
            .output()
            .expect("failed to create base dir");
        let path = String::from_utf8(path.stdout).expect("bad path");
        let mut bin_path = OsString::new();
        for binary_path in &self.binary_paths {
            let binary_path = &std::path::absolute(binary_path)
                .expect("bad path")
                .into_os_string();
            log::info!(target: "gadget", "Trying to spawn binary at {:?}", binary_path);
            self.custom_flags
                .insert("base-path".into(), Some(path.clone().into()));

            res = SubstrateNodeBuilder::try_spawn(binary_path, &self.custom_flags);
            if res.is_ok() {
                bin_path.clone_from(binary_path);
                break;
            }
        }

        let mut proc = match res {
            Ok(proc) => proc,
            Err(e) => return Err(Error::Io(e)),
        };

        // Create channels for log capturing
        let (init_tx, init_rx) = mpsc::channel();
        let (log_tx, log_rx) = mpsc::channel();

        // Take stderr for port detection and logging
        let stderr = proc.stderr.take().unwrap();
        let init_tx_clone = init_tx.clone();
        let log_tx_clone = log_tx.clone();

        // Spawn thread to handle stderr
        let stderr_handle = thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                log::debug!(target: "node-stderr", "{}", line);
                let _ = init_tx_clone.send(line.clone());
                let _ = log_tx_clone.send(line);
            }
        });

        // Take stdout for logging
        let stdout = proc.stdout.take().unwrap();
        let log_tx_stdout = log_tx.clone();

        // Spawn thread to handle stdout
        let stdout_handle = thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                log::debug!(target: "node-stdout", "{}", line);
                let _ = log_tx_stdout.send(line);
            }
        });

        // Process initialization logs with timeout
        let running_node = try_find_substrate_port_from_output(init_rx);

        let ws_port = running_node.ws_port()?;
        let p2p_address = running_node.p2p_address()?;
        let p2p_port = running_node.p2p_port()?;

        Ok(SubstrateNode {
            binary_path: bin_path,
            custom_flags: self.custom_flags,
            proc,
            ws_port,
            p2p_address,
            p2p_port,
            base_path: path,
            stdout_handle: Some(stdout_handle),
            stderr_handle: Some(stderr_handle),
            log_capture_tx: Some(log_tx),
        })
    }

    // Attempt to spawn a binary with the path/flags given.
    fn try_spawn(
        binary_path: &OsString,
        custom_flags: &HashMap<CowStr, Option<CowStr>>,
    ) -> Result<Child, std::io::Error> {
        let mut cmd = Command::new(binary_path);

        cmd.env("RUST_LOG", "info,libp2p_tcp=debug")
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .arg("--dev")
            .arg("--port=0");

        for (key, val) in custom_flags {
            let arg = match val {
                Some(val) => format!("--{key}={val}"),
                None => format!("--{key}"),
            };
            cmd.arg(arg);
        }

        gadget_sdk::trace!("Spawning node with command: {cmd:?}");
        cmd.spawn()
    }
}

pub struct SubstrateNode {
    binary_path: OsString,
    custom_flags: HashMap<CowStr, Option<CowStr>>,
    proc: process::Child,
    ws_port: u16,
    p2p_address: String,
    p2p_port: u32,
    base_path: String,
    stdout_handle: Option<thread::JoinHandle<()>>,
    stderr_handle: Option<thread::JoinHandle<()>>,
    log_capture_tx: Option<mpsc::Sender<String>>,
}

impl SubstrateNode {
    /// Configure and spawn a new [`SubstrateNode`].
    pub fn builder() -> SubstrateNodeBuilder {
        SubstrateNodeBuilder::new()
    }

    /// Return the ID of the running process.
    pub fn id(&self) -> u32 {
        self.proc.id()
    }

    /// Return the port that WS connections are accepted on.
    pub fn ws_port(&self) -> u16 {
        self.ws_port
    }

    /// Return the libp2p address of the running node.
    pub fn p2p_address(&self) -> String {
        self.p2p_address.clone()
    }

    /// Return the libp2p port of the running node.
    pub fn p2p_port(&self) -> u32 {
        self.p2p_port
    }

    /// Kill the process.
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.proc.kill()
    }

    /// restart the node, handing back an object which, when dropped, will stop it.
    pub fn restart(&mut self) -> Result<(), std::io::Error> {
        // First kill the existing process
        self.kill()?;

        // Drop existing log channels and handles
        self.log_capture_tx.take();
        if let Some(handle) = self.stdout_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.stderr_handle.take() {
            let _ = handle.join();
        }

        // Create new channels for logging
        let (log_tx, _log_rx) = mpsc::channel();

        // Spawn new process
        let mut proc = self.try_spawn()?;

        // Setup new logging for stdout
        if let Some(stdout) = proc.stdout.take() {
            let log_tx_clone = log_tx.clone();
            let handle = thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    log::debug!(target: "node-stdout", "{}", line);
                    let _ = log_tx_clone.send(line);
                }
            });
            self.stdout_handle = Some(handle);
        }

        // Setup new logging for stderr
        if let Some(stderr) = proc.stderr.take() {
            let log_tx_clone = log_tx.clone();
            let handle = thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    log::debug!(target: "node-stderr", "{}", line);
                    let _ = log_tx_clone.send(line);
                }
            });
            self.stderr_handle = Some(handle);
        }

        self.proc = proc;
        self.log_capture_tx = Some(log_tx);

        Ok(())
    }

    // Attempt to spawn a binary with the path/flags given.
    fn try_spawn(&mut self) -> Result<Child, std::io::Error> {
        let mut cmd = Command::new(&self.binary_path);

        cmd.env("RUST_LOG", "info,libp2p_tcp=debug")
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .arg("--dev");

        for (key, val) in &self.custom_flags {
            let arg = match val {
                Some(val) => format!("--{key}={val}"),
                None => format!("--{key}"),
            };
            cmd.arg(arg);
        }

        cmd.arg(format!("--rpc-port={}", self.ws_port));
        cmd.arg(format!("--port={}", self.p2p_port));

        log::debug!(target: "gadget", "Restarting node with command: {:?}", cmd);
        cmd.spawn()
    }

    fn setup_log_handling(&mut self) {
        if let Some(stdout) = self.proc.stdout.take() {
            let log_tx = self.log_capture_tx.clone();
            let handle = thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    log::debug!(target: "node-stdout", "{}", line);
                    if let Some(tx) = &log_tx {
                        let _ = tx.send(line);
                    }
                }
            });
            self.stdout_handle = Some(handle);
        }

        if let Some(stderr) = self.proc.stderr.take() {
            let log_tx = self.log_capture_tx.clone();
            let handle = thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    log::debug!(target: "node-stderr", "{}", line);
                    if let Some(tx) = &log_tx {
                        let _ = tx.send(line);
                    }
                }
            });
            self.stderr_handle = Some(handle);
        }
    }

    fn cleanup(&self) {
        let _ = Command::new("rm")
            .args(["-rf", &self.base_path])
            .output()
            .expect("success");
    }
}

impl Drop for SubstrateNode {
    fn drop(&mut self) {
        // First drop the log capture channel to signal threads to exit
        self.log_capture_tx.take();

        // Kill the process
        let _ = self.kill();

        // Wait for log handling threads to finish with timeout
        let timeout = std::time::Duration::from_secs(5);
        if let Some(handle) = self.stdout_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.stderr_handle.take() {
            let _ = handle.join();
        }

        self.cleanup()
    }
}

// Consume a stderr reader from a spawned substrate command and
// locate the port number that is logged out to it.
fn try_find_substrate_port_from_output(rx: mpsc::Receiver<String>) -> SubstrateNodeInfo {
    let mut port = None;
    let mut p2p_address = None;
    let mut p2p_port = None;
    let mut log = String::new();

    let timeout = std::time::Duration::from_secs(30);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        // Try to receive with a small timeout to avoid blocking forever
        let line = match rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(line) => line,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        log::debug!(target: "gadget-init", "{}", line);
        log.push_str(&line);
        log.push('\n');

        // Parse the port lines
        let line_port = line
            // oldest message:
            .rsplit_once("Listening for new connections on 127.0.0.1:")
            // slightly newer message:
            .or_else(|| line.rsplit_once("Running JSON-RPC WS server: addr=127.0.0.1:"))
            // newest message (jsonrpsee merging http and ws servers):
            .or_else(|| line.rsplit_once("Running JSON-RPC server: addr=127.0.0.1:"))
            // Sometimes the addr is 0.0.0.0
            .or_else(|| line.rsplit_once("Running JSON-RPC server: addr=0.0.0.0:"))
            .map(|(_, port_str)| port_str);

        if let Some(ports) = line_port {
            // If more than one rpc server is started the log will capture multiple ports
            // such as `addr=127.0.0.1:9944,[::1]:9944`
            let port_str: String = ports.chars().take_while(|c| c.is_numeric()).collect();

            // expect to have a number here (the chars after '127.0.0.1:') and parse them into a u16.
            let port_num = port_str
                .parse()
                .unwrap_or_else(|_| panic!("valid port expected for log line, got '{port_str}'"));
            port = Some(port_num);
        }

        // Parse the p2p address line
        let line_address = line
            .rsplit_once("Local node identity is: ")
            .map(|(_, address_str)| address_str);

        if let Some(line_address) = line_address {
            let address = line_address.trim_end_matches(|b: char| b.is_ascii_whitespace());
            p2p_address = Some(address.into());
        }

        // Parse the p2p port line (present in debug logs)
        let p2p_port_line = line
            .rsplit_once("New listen address: /ip4/127.0.0.1/tcp/")
            .map(|(_, address_str)| address_str);

        if let Some(line_port) = p2p_port_line {
            // trim non-numeric chars from the end of the port part of the line.
            let port_str = line_port.trim_end_matches(|b: char| !b.is_ascii_digit());

            // expect to have a number here (the chars after '127.0.0.1:') and parse them into a u16.
            let port_num = port_str
                .parse()
                .unwrap_or_else(|_| panic!("valid port expected for log line, got '{port_str}'"));
            p2p_port = Some(port_num);
        }

        if port.is_some() && p2p_address.is_some() && p2p_port.is_some() {
            break;
        }
    }

    SubstrateNodeInfo {
        ws_port: port,
        p2p_address,
        p2p_port,
        log,
    }
}

/// Data extracted from the running node's stdout.
#[derive(Debug)]
pub struct SubstrateNodeInfo {
    ws_port: Option<u16>,
    p2p_address: Option<String>,
    p2p_port: Option<u32>,
    log: String,
}

impl SubstrateNodeInfo {
    pub fn ws_port(&self) -> Result<u16, Error> {
        self.ws_port
            .ok_or_else(|| Error::CouldNotExtractPort(self.log.clone()))
    }

    pub fn p2p_address(&self) -> Result<String, Error> {
        self.p2p_address
            .clone()
            .ok_or_else(|| Error::CouldNotExtractP2pAddress(self.log.clone()))
    }

    pub fn p2p_port(&self) -> Result<u32, Error> {
        self.p2p_port
            .ok_or_else(|| Error::CouldNotExtractP2pPort(self.log.clone()))
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeConfig {
    pub use_local_tangle: bool,
    pub log_level: Option<String>,
    pub log_targets: Vec<(String, String)>,
}

impl NodeConfig {
    pub fn new(use_local_tangle: bool) -> Self {
        Self {
            use_local_tangle,
            log_level: None,
            log_targets: Vec::new(),
        }
    }

    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = Some(level.into());
        self
    }

    pub fn with_log_target(mut self, target: impl Into<String>, level: impl Into<String>) -> Self {
        self.log_targets.push((target.into(), level.into()));
        self
    }

    pub fn to_log_string(&self) -> String {
        let mut parts = Vec::new();

        // Add global level if set
        if let Some(level) = &self.log_level {
            parts.push(level.clone());
        }

        // Add target-specific levels
        for (target, level) in &self.log_targets {
            parts.push(format!("{}={}", target, level));
        }

        parts.join(",")
    }
}
