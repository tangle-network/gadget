use crate::config::BlueprintManagerConfig;
use crate::gadget::native::FilteredBlueprint;
use crate::gadget::ActiveGadgets;
use crate::sdk::async_trait;
use crate::sources::BinarySourceFetcher;
use color_eyre::Report;
use gadget_common::prelude::DebugLogger;
use gadget_io::GadgetConfig;
use std::path::PathBuf;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::TestFetcher;

pub struct TestSourceFetcher<'a> {
    pub fetcher: &'a TestFetcher,
    pub blueprint_id: u64,
    pub logger: &'a DebugLogger,
    pub gadget_name: String,
}

#[async_trait]
impl BinarySourceFetcher for TestSourceFetcher<'_> {
    async fn get_binary(&self) -> color_eyre::Result<PathBuf> {
        // Step 1: Build the binary. It will be stored in the root directory/bin/
        let TestFetcher {
            cargo_bin,
            base_path,
            ..
        } = self.fetcher;
        let cargo_bin = String::from_utf8(cargo_bin.0 .0.clone())
            .map_err(|err| Report::msg(format!("Failed to parse `cargo_bin`: {:?}", err)))?;
        let base_path = String::from_utf8(base_path.0 .0.clone())
            .map_err(|err| Report::msg(format!("Failed to parse `base_path`: {:?}", err)))?;
        let binary_path = format!("{base_path}/bin/{cargo_bin}");
        let path = PathBuf::from(binary_path);
        let path = path
            .canonicalize()
            .map_err(|err| Report::msg(format!("Failed to canonicalize path: {:?}", err)))?;

        let tokio_build_process = tokio::process::Command::new("cargo")
            .arg("install")
            .arg("--path")
            .arg(&base_path)
            .arg("--bin")
            .arg(cargo_bin)
            .arg("--root")
            .arg(&base_path)
            .stdout(std::process::Stdio::inherit()) // Inherit the stdout of this process
            .stderr(std::process::Stdio::inherit()) // Inherit the stderr of this process
            .stdin(std::process::Stdio::null())
            .current_dir(&std::env::current_dir()?)
            .output()
            .await
            .map_err(|err| Report::msg(format!("Failed to run `cargo install`: {:?}", err)))?;

        if !tokio_build_process.status.success() {
            return Err(Report::msg(format!(
                "Failed to build binary: {:?}",
                tokio_build_process
            )));
        }

        if !path.exists() {
            return Err(Report::msg(format!("Binary not found at path: {:?}", path)));
        }

        Ok(path)
    }

    fn blueprint_id(&self) -> u64 {
        self.blueprint_id
    }

    fn name(&self) -> String {
        self.gadget_name.clone()
    }
}
