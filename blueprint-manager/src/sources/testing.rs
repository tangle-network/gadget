use crate::sources::BinarySourceFetcher;
use async_trait::async_trait;
use color_eyre::Report;
use gadget_sdk::logger::Logger;
use std::path::PathBuf;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::TestFetcher;

pub struct TestSourceFetcher<'a> {
    pub fetcher: TestFetcher,
    pub blueprint_id: u64,
    pub logger: &'a Logger,
    pub gadget_name: String,
}

#[async_trait]
impl BinarySourceFetcher for TestSourceFetcher<'_> {
    async fn get_binary(&self) -> color_eyre::Result<PathBuf> {
        // Step 1: Build the binary. It will be stored in the root directory/bin/
        let TestFetcher {
            cargo_package,
            base_path,
            ..
        } = &self.fetcher;
        let cargo_bin = String::from_utf8(cargo_package.0 .0.clone())
            .map_err(|err| Report::msg(format!("Failed to parse `cargo_bin`: {:?}", err)))?;
        let base_path_str = String::from_utf8(base_path.0 .0.clone())
            .map_err(|err| Report::msg(format!("Failed to parse `base_path`: {:?}", err)))?;
        let git_repo_root = get_git_repo_root_path().await?;

        let base_path = std::path::absolute(git_repo_root.join(&base_path_str))?;
        let binary_path = git_repo_root.join(&base_path).join("bin").join(&cargo_bin);
        let binary_path = std::path::absolute(&binary_path)?;

        self.logger
            .trace(format!("Base Path: {}", base_path.display()));
        self.logger
            .trace(format!("Binary Path: {}", binary_path.display()));
        self.logger.info("Building binary...");

        let env = std::env::vars().collect::<Vec<(String, String)>>();

        // Note: even if multiple gadgets are built, only the leader will actually build
        // while the followers will just hang on the Cargo.lock file and then instantly
        // finish compilation
        let tokio_build_process = tokio::process::Command::new("cargo")
            .arg("install")
            .arg("--path")
            .arg(&base_path)
            //            .arg("--bin")
            //            .arg(cargo_bin)
            .arg("--root")
            .arg(&base_path)
            .stdout(std::process::Stdio::inherit()) // Inherit the stdout of this process
            .stderr(std::process::Stdio::inherit()) // Inherit the stderr of this process
            .stdin(std::process::Stdio::null())
            .current_dir(&std::env::current_dir()?)
            .envs(env)
            .output()
            .await
            .map_err(|err| Report::msg(format!("Failed to run `cargo install`: {:?}", err)))?;

        if !tokio_build_process.status.success() {
            return Err(Report::msg(format!(
                "Failed to build binary: {:?}",
                tokio_build_process
            )));
        }

        if !binary_path.exists() {
            return Err(Report::msg(format!(
                "Binary not found at path: {}",
                binary_path.display()
            )));
        }

        self.logger.info(format!(
            "Successfully built binary to {}",
            binary_path.display()
        ));

        Ok(binary_path)
    }

    fn blueprint_id(&self) -> u64 {
        self.blueprint_id
    }

    fn name(&self) -> String {
        self.gadget_name.clone()
    }
}
async fn get_git_repo_root_path() -> color_eyre::Result<PathBuf> {
    // Run a process the determine thw root directory for this repo
    let output = tokio::process::Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .await?;

    if !output.status.success() {
        return Err(Report::msg(format!(
            "Failed to get git root path: {:?}",
            output
        )));
    }

    Ok(PathBuf::from(String::from_utf8(output.stdout)?.trim()))
}
