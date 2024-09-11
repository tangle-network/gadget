use crate::gadget::native::get_gadget_binary;
use crate::sdk;
use crate::sdk::utils::{
    get_download_url, hash_bytes_to_hex, is_windows, msg_to_error, valid_file_exists,
};
use async_trait::async_trait;
// use gadget_blueprint_proc_macro_core::GithubFetcher;
use crate::sources::BinarySourceFetcher;
use color_eyre::eyre::OptionExt;
use gadget_sdk::logger::Logger;
use std::path::PathBuf;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::GithubFetcher;
use tokio::io::AsyncWriteExt;

pub struct GithubBinaryFetcher<'a> {
    pub fetcher: GithubFetcher,
    pub blueprint_id: u64,
    pub logger: &'a Logger,
    pub gadget_name: String,
}

#[async_trait]
impl BinarySourceFetcher for GithubBinaryFetcher<'_> {
    async fn get_binary(&self) -> color_eyre::Result<PathBuf> {
        let relevant_binary = get_gadget_binary(&self.fetcher.binaries.0)
            .ok_or_eyre("Unable to find matching binary")?;
        let expected_hash = sdk::utils::slice_32_to_sha_hex_string(relevant_binary.sha256);
        let current_dir = std::env::current_dir()?;
        let mut binary_download_path =
            format!("{}/protocol-{:?}", current_dir.display(), self.fetcher.tag);

        if is_windows() {
            binary_download_path += ".exe"
        }

        self.logger
            .info(format!("Downloading to {binary_download_path}"));

        // Check if the binary exists, if not download it
        let retrieved_hash = if !valid_file_exists(&binary_download_path, &expected_hash).await {
            let url = get_download_url(relevant_binary, &self.fetcher);

            let download = reqwest::get(&url)
                .await
                .map_err(|err| msg_to_error(err.to_string()))?
                .bytes()
                .await
                .map_err(|err| msg_to_error(err.to_string()))?;
            let retrieved_hash = hash_bytes_to_hex(&download);

            // Write the binary to disk
            let mut file = tokio::fs::File::create(&binary_download_path).await?;
            file.write_all(&download).await?;
            file.flush().await?;
            Some(retrieved_hash)
        } else {
            None
        };

        if let Some(retrieved_hash) = retrieved_hash {
            if retrieved_hash.trim() != expected_hash.trim() {
                self.logger.error(format!(
                    "Binary hash {} mismatched expected hash of {} for protocol: {}",
                    retrieved_hash, expected_hash, self.gadget_name
                ));
                return Ok(PathBuf::from(binary_download_path));
            }
        }

        Err(color_eyre::Report::msg(
            "The hash of the downloaded binary did not match",
        ))
    }

    fn blueprint_id(&self) -> u64 {
        self.blueprint_id
    }

    fn name(&self) -> String {
        self.gadget_name.clone()
    }
}
