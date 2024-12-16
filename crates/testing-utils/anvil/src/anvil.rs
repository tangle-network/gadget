use gadget_std::path::{Path, PathBuf};
use testcontainers::{
    core::{ExecCommand, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};
use tokio::io::AsyncBufReadExt;

const ANVIL_IMAGE: &str = "ghcr.io/foundry-rs/foundry";
const ANVIL_TAG: &str = "nightly-5b7e4cb3c882b28f3c32ba580de27ce7381f415a";

fn workspace_dir() -> PathBuf {
    let output = gadget_std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .unwrap()
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).unwrap().trim());
    cargo_path.parent().unwrap().to_path_buf()
}

/// Start an Anvil container for testing with contract state loaded.
pub async fn start_anvil_container(
    state_path: &str,
    include_logs: bool,
) -> (ContainerAsync<GenericImage>, String, String) {
    let relative_path = PathBuf::from(state_path);
    let absolute_path = workspace_dir().join(relative_path);
    let absolute_path_str = absolute_path.to_str().unwrap();

    if !absolute_path.exists() {
        panic!("Anvil state file not found at: {}", absolute_path.display());
    }

    let container = GenericImage::new(ANVIL_IMAGE, ANVIL_TAG)
        .with_wait_for(WaitFor::message_on_stdout("Listening on"))
        .with_exposed_port(8545.tcp())
        .with_entrypoint("anvil")
        .with_mount(testcontainers::core::Mount::bind_mount(
            absolute_path_str,
            "/testnet_state.json",
        ))
        .with_cmd([
            "--host",
            "0.0.0.0",
            "--load-state",
            "/testnet_state.json",
            "--base-fee",
            "0",
            "--gas-price",
            "0",
        ])
        .start()
        .await
        .expect("Error starting anvil container");

    if include_logs {
        let reader = container.stdout(true);
        tokio::task::spawn(async move {
            let mut reader = reader;
            let mut buffer = String::new();
            while reader.read_line(&mut buffer).await.unwrap() > 0 {
                println!("{:?}", buffer);
                buffer.clear();
            }
        });
    }

    let port = container
        .ports()
        .await
        .unwrap()
        .map_to_host_port_ipv4(8545)
        .unwrap();

    let http_endpoint = format!("http://localhost:{}", port);
    println!("Anvil HTTP endpoint: {}", http_endpoint);
    let ws_endpoint = format!("ws://localhost:{}", port);
    println!("Anvil WS endpoint: {}", ws_endpoint);

    mine_anvil_blocks(&container, 200).await;

    (container, http_endpoint, ws_endpoint)
}

/// Mine Anvil blocks.
pub async fn mine_anvil_blocks(container: &ContainerAsync<GenericImage>, n: u32) {
    let mut output = container
        .exec(ExecCommand::new([
            "cast",
            "rpc",
            "anvil_mine",
            n.to_string().as_str(),
        ]))
        .await
        .expect("Failed to mine anvil blocks");

    // blocking operation until the mining execution finishes
    output.stdout_to_vec().await.unwrap();
    assert_eq!(output.exit_code().await.unwrap().unwrap(), 0);
}

pub const ANVIL_PRIVATE_KEYS: [&str; 10] = [
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
    "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a",
    "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6",
    "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a",
    "0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba",
    "0x92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e",
    "0x4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356",
    "0xdbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97",
    "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6",
];
