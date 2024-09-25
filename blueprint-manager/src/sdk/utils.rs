use crate::config::BlueprintManagerConfig;
use crate::protocols::resolver::NativeGithubMetadata;
use gadget_io::GadgetConfig;
use gadget_sdk::config::Protocol;
use gadget_sdk::{info, warn};
use sha2::Digest;
use std::path::Path;
use std::string::FromUtf8Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::BoundedString;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::{
    GadgetBinary, GithubFetcher,
};

pub fn github_fetcher_to_native_github_metadata(
    gh: &GithubFetcher,
    blueprint_id: u64,
) -> NativeGithubMetadata {
    let owner = bytes_to_utf8_string(gh.owner.0 .0.clone()).expect("Should be valid");
    let repo = bytes_to_utf8_string(gh.repo.0 .0.clone()).expect("Should be valid");
    let tag = bytes_to_utf8_string(gh.tag.0 .0.clone()).expect("Should be valid");
    let git = format!("https://github.com/{owner}/{repo}");

    NativeGithubMetadata {
        fetcher: gh.clone(),
        git,
        tag,
        repo,
        owner,
        gadget_binaries: gh.binaries.0.clone(),
        blueprint_id,
    }
}

pub fn bounded_string_to_string(string: BoundedString) -> Result<String, FromUtf8Error> {
    let bytes: &Vec<u8> = &string.0 .0;
    String::from_utf8(bytes.clone())
}

pub fn generate_process_arguments(
    gadget_config: &GadgetConfig,
    opt: &BlueprintManagerConfig,
    blueprint_id: u64,
    service_id: u64,
    protocol: Protocol,
) -> color_eyre::Result<Vec<String>> {
    let mut arguments = vec![];
    arguments.push("run".to_string());

    if opt.test_mode {
        arguments.push("--test-mode".to_string());
    }

    for bootnode in &gadget_config.bootnodes {
        arguments.push(format!("--bootnodes={}", bootnode));
    }

    arguments.extend([
        format!("--bind-addr={}", gadget_config.bind_addr),
        format!("--bind-port={}", gadget_config.bind_port),
        format!("--url={}", gadget_config.url),
        format!("--keystore-uri={}", gadget_config.keystore_uri),
        format!("--chain={}", gadget_config.chain),
        format!("--verbose={}", opt.verbose),
        format!("--pretty={}", opt.pretty),
        format!("--blueprint-id={}", blueprint_id),
        format!("--service-id={}", service_id),
        format!("--protocol={}", protocol),
    ]);

    if let Some(keystore_password) = &gadget_config.keystore_password {
        arguments.push(format!("--keystore-password={}", keystore_password));
    }

    Ok(arguments)
}

pub fn hash_bytes_to_hex<T: AsRef<[u8]>>(input: T) -> String {
    let mut hasher = sha2::Sha256::default();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

pub async fn valid_file_exists(path: &str, expected_hash: &str) -> bool {
    // The hash is sha3_256 of the binary
    if let Ok(file) = gadget_io::tokio::fs::read(path).await {
        // Compute the SHA3-256
        let retrieved_bytes = hash_bytes_to_hex(file);
        expected_hash == retrieved_bytes.as_str()
    } else {
        false
    }
}

pub fn get_formatted_os_string() -> String {
    let os = std::env::consts::OS;

    match os {
        "macos" => "apple-darwin".to_string(),
        "windows" => "pc-windows-msvc".to_string(),
        "linux" => "unknown-linux-gnu".to_string(),
        _ => os.to_string(),
    }
}

pub fn get_download_url(binary: &GadgetBinary, fetcher: &GithubFetcher) -> String {
    let os = get_formatted_os_string();
    let ext = if os == "windows" { ".exe" } else { "" };
    let owner = String::from_utf8(fetcher.owner.0 .0.clone()).expect("Should be a valid owner");
    let repo = String::from_utf8(fetcher.repo.0 .0.clone()).expect("Should be a valid repo");
    let tag = String::from_utf8(fetcher.tag.0 .0.clone()).expect("Should be a valid tag");
    let binary_name =
        String::from_utf8(binary.name.0 .0.clone()).expect("Should be a valid binary name");
    let os_name = format!("{:?}", binary.os).to_lowercase();
    let arch_name = format!("{:?}", binary.arch).to_lowercase();
    // https://github.com/<owner>/<repo>/releases/download/v<tag>/<path>
    format!("https://github.com/{owner}/{repo}/releases/download/v{tag}/{binary_name}-{os_name}-{arch_name}{ext}")
}

pub fn msg_to_error<T: Into<String>>(msg: T) -> color_eyre::Report {
    color_eyre::Report::msg(msg.into())
}

pub fn get_service_str(svc: &NativeGithubMetadata) -> String {
    let repo = svc.git.clone();
    let vals: Vec<&str> = repo.split(".com/").collect();
    vals[1].to_string()
}

pub async fn chmod_x_file<P: AsRef<Path>>(path: P) -> color_eyre::Result<()> {
    let success = gadget_io::tokio::process::Command::new("chmod")
        .arg("+x")
        .arg(format!("{}", path.as_ref().display()))
        .spawn()?
        .wait_with_output()
        .await?
        .status
        .success();

    if success {
        Ok(())
    } else {
        Err(color_eyre::eyre::eyre!(
            "Failed to chmod +x {}",
            path.as_ref().display()
        ))
    }
}

pub fn is_windows() -> bool {
    std::env::consts::OS == "windows"
}

pub fn generate_running_process_status_handle(
    process: gadget_io::tokio::process::Child,
    service_name: &str,
) -> (Arc<AtomicBool>, gadget_io::tokio::sync::oneshot::Sender<()>) {
    let (stop_tx, stop_rx) = gadget_io::tokio::sync::oneshot::channel::<()>();
    let status = Arc::new(AtomicBool::new(true));
    let status_clone = status.clone();
    let service_name = service_name.to_string();

    let task = async move {
        info!("Starting process execution for {service_name}");
        let output = process.wait_with_output().await;
        warn!("Process for {service_name} exited: {output:?}");
        status_clone.store(false, Ordering::Relaxed);
    };

    let task = async move {
        gadget_io::tokio::select! {
            _ = stop_rx => {},
            _ = task => {},
        }
    };

    gadget_io::tokio::spawn(task);
    (status, stop_tx)
}

pub fn bytes_to_utf8_string<T: Into<Vec<u8>>>(input: T) -> color_eyre::Result<String> {
    String::from_utf8(input.into()).map_err(|err| msg_to_error(err.to_string()))
}

pub fn slice_32_to_sha_hex_string(hash: [u8; 32]) -> String {
    use std::fmt::Write;
    hash.iter().fold(String::new(), |mut acc, byte| {
        write!(&mut acc, "{:02x}", byte).expect("Should be able to write");
        acc
    })
}
