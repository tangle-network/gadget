use crate::config::ShellManagerOpts;
use crate::protocols::resolver::NativeGithubMetadata;
use gadget_common::config::DebugLogger;
use gadget_common::sp_core::H256;
use gadget_common::tangle_runtime::AccountId32;
use gadget_io::{defaults, ShellTomlConfig};
use sha2::Digest;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tangle_environment::api::ServicesClient;
use tangle_subxt::subxt::backend::BlockRef;
use tangle_subxt::subxt::Config;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::{
    Gadget, GadgetSourceFetcher, GithubFetcher, ServiceBlueprint,
};

pub async fn get_blueprints<C: Config>(
    runtime: &ServicesClient<C>,
    block_hash: [u8; 32],
    account_id: AccountId32,
    global_protocols: &[NativeGithubMetadata],
    test_mode: bool,
) -> color_eyre::Result<Vec<ServiceBlueprint>>
where
    BlockRef<<C as Config>::Hash>: From<BlockRef<H256>>,
{
    if test_mode {
        return Ok(global_protocols.iter().map(|r| r.clone()).collect());
    }

    perform_query(
        runtime,
        block_hash,
        account_id,
        get_blueprints_native_fetcher,
    )
    .await
}

pub async fn get_services<C: Config>(
    runtime: &ServicesClient<C>,
    block_hash: [u8; 32],
    account_id: AccountId32,
    global_protocols: &[NativeGithubMetadata],
    test_mode: bool,
) -> color_eyre::Result<Vec<(GithubFetcher, NativeGithubMetadata)>>
where
    BlockRef<<C as Config>::Hash>: From<BlockRef<H256>>,
{
    if test_mode {
        return Ok(global_protocols
            .iter()
            .map(|r| {
                let metadata = r.clone();
                let fetcher = GithubFetcher {
                    owner: metadata.owner.clone().try_into().unwrap(),
                    repo: metadata.repo.clone().try_into().unwrap(),
                    tag: metadata.tag.clone().try_into().unwrap(),
                    binaries: BoundedVec(),
                };
            })
            .collect());
    }

    perform_query(runtime, block_hash, account_id, get_native_metadata_fetcher).await
}

async fn perform_query<C: Config, K, F: Fn(Vec<ServiceBlueprint>) -> Vec<K>>(
    runtime: &ServicesClient<C>,
    block_hash: [u8; 32],
    account_id: AccountId32,
    remapping: F,
) -> color_eyre::Result<Vec<K>>
where
    BlockRef<<C as Config>::Hash>: From<BlockRef<H256>>,
{
    runtime
        .query_restaker_blueprints(block_hash, account_id)
        .await
        .map_err(|err| msg_to_error(err.to_string()))
        .map(|r| remapping(r))
        .collect()
}

/// Simple identity transformation
fn get_blueprints_native_fetcher(
    service_blueprints: Vec<ServiceBlueprint>,
) -> Vec<ServiceBlueprint> {
    service_blueprints
}

fn get_native_metadata_fetcher(
    service_blueprints: Vec<ServiceBlueprint>,
) -> Vec<(GithubFetcher, NativeGithubMetadata)> {
    let mut ret = vec![];
    for service_blueprint in service_blueprints {
        if let Gadget::Native(gadget) = service_blueprint {
            let source = &gadget.soruces.0[0];
            if let GadgetSourceFetcher::Github(gh) = &source.fetcher {
                ret.push((gh.clone(), github_fetcher_to_native_github_metadata(gh)));
            }
        }
    }

    ret
}

pub fn github_fetcher_to_native_github_metadata(gh: &GithubFetcher) -> NativeGithubMetadata {
    let owner = bytes_to_utf8_string(gh.owner.0 .0.clone()).expect("Should be valid");
    let repo = bytes_to_utf8_string(gh.repo.0 .0.clone())?;
    let tag = bytes_to_utf8_string(gh.tag.0 .0.clone())?;
    let git = format!("https://github.com/{owner}/{repo}");

    NativeGithubMetadata {
        git,
        tag,
        repo,
        owner,
        gadget_binaries: gh.binaries.0.clone(),
    }
}

pub fn generate_process_arguments(
    shell_config: &ShellTomlConfig,
    opt: &ShellManagerOpts,
) -> color_eyre::Result<Vec<String>> {
    let mut arguments = vec![
        format!("--bind-ip={}", shell_config.bind_ip),
        format!("--url={}", shell_config.url),
        format!(
            "--bootnodes={}",
            shell_config
                .bootnodes
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<String>>()
                .join(",")
        ),
        format!(
            "--node-key={}",
            shell_config
                .node_key
                .clone()
                .unwrap_or_else(|| { hex::encode(defaults::generate_node_key()) })
        ),
        format!("--base-path={}", shell_config.base_path.display()),
        format!("--chain={}", shell_config.chain.to_string()),
        format!("--verbose={}", opt.verbose),
        format!("--pretty={}", opt.pretty),
    ];

    if let Some(keystore_password) = &shell_config.keystore_password {
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

pub fn get_download_url<T: Into<String>>(git: T, tag: &str) -> String {
    let os = get_formatted_os_string();
    let arch = std::env::consts::ARCH;

    let mut git = git.into();

    // Ensure the first part of the url ends with `/`
    if git.ends_with(".git") {
        git = git.replace(".git", "/")
    }

    if !git.ends_with('/') {
        git.push('/')
    }

    let ext = if os == "windows" { ".exe" } else { "" };
    // https://github.com/<owner>/<repo>/releases/download/v<tag>/<path>
    format!("{git}releases/download/v{tag}/protocol-{os}-{arch}{ext}")
}

pub fn msg_to_error<T: Into<String>>(msg: T) -> color_eyre::Report {
    color_eyre::Report::msg(msg.into())
}

pub fn get_service_str(svc: &NativeGithubMetadata) -> String {
    let repo = svc.git.clone();
    let vals = repo.split(".com/").collect();
    vals[1].clone()
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
    logger: &DebugLogger,
    role_type: &str,
) -> (Arc<AtomicBool>, gadget_io::tokio::sync::oneshot::Sender<()>) {
    let (stop_tx, stop_rx) = gadget_io::tokio::sync::oneshot::channel::<()>();
    let status = Arc::new(AtomicBool::new(true));
    let status_clone = status.clone();
    let logger = logger.clone();
    let role_type = role_type.to_string();

    let task = async move {
        let output = process.wait_with_output().await;
        logger.warn(format!("Process for {role_type} exited: {output:?}"));
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
