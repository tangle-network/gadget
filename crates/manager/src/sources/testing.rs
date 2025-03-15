use crate::error::{Error, Result};
use crate::sources::BinarySourceFetcher;
use blueprint_core::trace;
use std::path::PathBuf;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::gadget::TestFetcher;

pub struct TestSourceFetcher {
    pub fetcher: TestFetcher,
    pub blueprint_id: u64,
    pub gadget_name: String,
}

impl BinarySourceFetcher for TestSourceFetcher {
    async fn get_binary(&self) -> Result<PathBuf> {
        let TestFetcher {
            cargo_package,
            base_path,
            ..
        } = &self.fetcher;
        let cargo_bin = String::from_utf8(cargo_package.0.0.clone())
            .map_err(|err| Error::Other(format!("Failed to parse `cargo_bin`: {:?}", err)))?;
        let base_path_str = String::from_utf8(base_path.0.0.clone())
            .map_err(|err| Error::Other(format!("Failed to parse `base_path`: {:?}", err)))?;
        let git_repo_root = get_git_repo_root_path().await?;

        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let base_path = std::path::absolute(git_repo_root.join(&base_path_str))?;

        let target_dir = match std::env::var("CARGO_TARGET_DIR") {
            Ok(target) => PathBuf::from(target),
            Err(_) => git_repo_root.join(&base_path).join("target"),
        };

        let binary_path = target_dir.join(profile).join(&cargo_bin);
        let binary_path = std::path::absolute(&binary_path)?;

        trace!("Base Path: {}", base_path.display());
        trace!("Binary Path: {}", binary_path.display());

        // Run cargo build on the cargo_bin and ensure it build to the binary_path
        let mut command = tokio::process::Command::new("cargo");
        command
            .arg("build")
            .arg(format!("--target-dir={}", target_dir.display()))
            .arg("--bin")
            .arg(&cargo_bin);

        if !cfg!(debug_assertions) {
            command.arg("--release");
        }

        trace!("Running build command in {}", base_path.display());
        let output = command.current_dir(&base_path).output().await.unwrap();
        trace!("Build command run");
        if !output.status.success() {
            blueprint_core::warn!("Failed to build binary");
            return Err(Error::BuildBinary(output));
        }
        trace!("Successfully built binary");

        Ok(binary_path)
    }

    fn blueprint_id(&self) -> u64 {
        self.blueprint_id
    }

    fn name(&self) -> String {
        self.gadget_name.clone()
    }
}
async fn get_git_repo_root_path() -> Result<PathBuf> {
    // Run a process to determine the root directory for this repo
    let output = tokio::process::Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .await?;

    if !output.status.success() {
        return Err(Error::FetchGitRoot(output));
    }

    Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()))
}
