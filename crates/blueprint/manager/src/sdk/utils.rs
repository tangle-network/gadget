use crate::protocols::resolver::NativeGithubMetadata;
use gadget_logging::{info, warn};
use sha2::Digest;
use std::path::Path;
use std::string::FromUtf8Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::BoundedString;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::{
    GadgetBinary, GithubFetcher,
};

pub fn bounded_string_to_string(string: BoundedString) -> Result<String, FromUtf8Error> {
    let bytes: &Vec<u8> = &string.0 .0;
    String::from_utf8(bytes.clone())
}

pub fn hash_bytes_to_hex<T: AsRef<[u8]>>(input: T) -> String {
    let mut hasher = sha2::Sha256::default();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

pub async fn valid_file_exists(path: &str, expected_hash: &str) -> bool {
    // The hash is sha3_256 of the binary
    if let Ok(file) = tokio::fs::read(path).await {
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

pub async fn chmod_x_file<P: AsRef<Path>>(path: P) -> color_eyre::Result<()> {
    let success = tokio::process::Command::new("chmod")
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
    process: tokio::process::Child,
    service_name: &str,
) -> (Arc<AtomicBool>, tokio::sync::oneshot::Sender<()>) {
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
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
        tokio::select! {
            _ = stop_rx => {},
            _ = task => {},
        }
    };

    tokio::spawn(task);
    (status, stop_tx)
}

pub fn slice_32_to_sha_hex_string(hash: [u8; 32]) -> String {
    use std::fmt::Write;
    hash.iter().fold(String::new(), |mut acc, byte| {
        write!(&mut acc, "{:02x}", byte).expect("Should be able to write");
        acc
    })
}
