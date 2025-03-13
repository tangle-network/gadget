use crate::node::testnet::{Error, SubstrateNode, TANGLE_NODE_ENV};
use gadget_std::fs;
use gadget_std::io::Write;
use gadget_std::path::PathBuf;
use reqwest;

pub mod testnet;
pub mod transactions;

pub use testnet::NodeConfig;

const TANGLE_RELEASE_MAC: &str = "https://github.com/tangle-network/tangle/releases/download/6174770/tangle-testnet-manual-seal-darwin-amd64";
const TANGLE_RELEASE_LINUX: &str = "https://github.com/tangle-network/tangle/releases/download/6174770/tangle-testnet-manual-seal-linux-amd64";

/// Downloads the appropriate Tangle binary for the current platform and returns the path
///
/// # Errors
///
/// * Unsupported platform
/// * Unable to determine the user's cache directory
pub async fn download_tangle_binary() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let download_url = if cfg!(target_os = "macos") {
        TANGLE_RELEASE_MAC
    } else if cfg!(target_os = "linux") {
        TANGLE_RELEASE_LINUX
    } else {
        return Err("Unsupported platform".into());
    };

    // Create cache directory in user's home directory
    let cache_dir = dirs::cache_dir()
        .ok_or("Could not determine cache directory")?
        .join("tangle-binary");
    fs::create_dir_all(&cache_dir)?;

    let binary_path = cache_dir.join("tangle");

    let version_path = cache_dir.join("version.txt");
    let commit_hash = download_url.split('/').nth(7).unwrap_or_default();

    let should_download = if binary_path.exists() && version_path.exists() {
        // Check if version matches
        let stored_version = fs::read_to_string(&version_path)?;
        stored_version.trim() != commit_hash
    } else {
        true
    };

    if should_download {
        tracing::info!("Downloading Tangle binary...");

        let response = reqwest::get(download_url).await?;
        let bytes = response.bytes().await?;

        let mut file = fs::File::create(&binary_path)?;
        file.write_all(&bytes)?;

        // Make binary executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&binary_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary_path, perms)?;
        }

        // Write version file
        fs::write(&version_path, commit_hash)?;
    }

    Ok(binary_path)
}

/// Run a Tangle node with the given configuration.
///
/// The node will shut down when the returned handle is dropped.
///
/// # Errors
///
/// * If not using a local node, see [`download_tangle_binary`]
pub async fn run(config: NodeConfig) -> Result<SubstrateNode, Error> {
    let mut builder = SubstrateNode::builder();

    // Add binary paths
    if config.use_local_tangle {
        if let Ok(tangle_from_env) = std::env::var(TANGLE_NODE_ENV) {
            builder.add_binary_path(tangle_from_env);
        }

        builder
            .add_binary_path("../tangle/target/release/tangle")
            .add_binary_path("../../tangle/target/release/tangle")
            .add_binary_path("../../../tangle/target/release/tangle");
    } else {
        let binary_path = download_tangle_binary()
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e.to_string())))?;
        builder.add_binary_path(binary_path.to_string_lossy().to_string());
    }

    // Add standard arguments
    builder
        .arg("validator")
        .arg_val("rpc-cors", "all")
        .arg_val("rpc-methods", "unsafe")
        .arg("rpc-external")
        .arg_val("sealing", "manual");

    // Add log configuration
    let log_string = config.to_log_string();
    if !log_string.is_empty() {
        builder.arg_val("log", log_string);
    }

    builder.spawn()
}

#[macro_export]
/// A template that makes creating domain-specific macros for tangle-based blueprints easier
macro_rules! tangle_blueprint_test_template {
    ($N:tt, $test_logic:expr, $node_config:expr,) => {
        use $crate::test_ext::new_test_ext_blueprint_manager;

        #[::gadget_sdk::tokio::test(flavor = "multi_thread", crate = "::gadget_sdk::tokio")]
        async fn test_blueprint() {
            ::blueprint_test_utils::setup_log();

            let tmp_dir = $crate::tempfile::TempDir::new().unwrap();
            let tmp_dir_path = tmp_dir.path().to_string_lossy().into_owned();

            ::blueprint_test_utils::test_ext::new_test_ext_blueprint_manager::<$N, 1, String, _, _>(
                tmp_dir_path,
                ::blueprint_test_utils::run_test_blueprint_manager,
                $node_config,
            )
            .await
            .execute_with_async($test_logic)
            .await
        }
    };
}

#[macro_export]
macro_rules! test_tangle_blueprint {
    (
        $N:tt,
        $T:tt,
        $job_id:tt,
        [$($inputs:expr),*],
        [$($expected_output:expr),*],
        $call_id:expr,
        $node_config:expr,
    ) => {
        ::blueprint_test_utils::tangle_blueprint_test_template!(
            $N,
            |client, handles, blueprint, _| async move {
                let keypair = handles[0].sr25519_id().clone();
                let selected_service = &blueprint.services[0];
                let service_id = selected_service.id;

                ::gadget_sdk::info!(
                    "Submitting job {} with service ID {service_id}", $job_id
                );

                let job_args = vec![$($inputs),*];

                let job = ::blueprint_test_utils::submit_job(
                    client,
                    &keypair,
                    service_id,
                    $job_id as ::blueprint_test_utils::Job,
                    job_args,
                    $call_id,
                )
                .await
                .expect("Failed to submit job");

                let call_id = job.call_id;

                ::gadget_sdk::info!(
                    "Submitted job {} with service ID {service_id} has call id {call_id}", $job_id
                );

                let job_results = ::blueprint_test_utils::wait_for_completion_of_tangle_job(client, service_id, call_id, $T)
                    .await
                    .expect("Failed to wait for job completion");

                assert_eq!(job_results.service_id, service_id);
                assert_eq!(job_results.call_id, call_id);

                let expected_outputs = vec![$($expected_output),*];
                if expected_outputs.is_empty() {
                    ::gadget_sdk::info!("No expected outputs specified, skipping verification");
                    return
                }

                assert_eq!(job_results.result.len(), expected_outputs.len(), "Number of outputs doesn't match expected");

                for (result, expected) in job_results.result.into_iter().zip(expected_outputs.into_iter()) {
                    assert_eq!(result, expected);
                }
            },
            $node_config,
        );
    };
    (
        $N:tt,
        $job_id:tt,
        [$($input:expr),*],
        [$($expected_output:expr),*]
        $call_id:expr,
    ) => {
        ::blueprint_test_utils::test_tangle_blueprint!($N, $N, $job_id, [$($input),+], [$($expected_output),+], $call_id);
    };
}
