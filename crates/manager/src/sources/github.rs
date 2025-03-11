use crate::error::{Error, Result};
use crate::gadget::native::get_gadget_binary;
use crate::sdk;
use crate::sdk::utils::{get_download_url, hash_bytes_to_hex, valid_file_exists};
use crate::sources::BinarySourceFetcher;
use gadget_logging::info;
use std::path::PathBuf;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::gadget::GithubFetcher;
use tokio::io::AsyncWriteExt;

pub struct GithubBinaryFetcher {
    pub fetcher: GithubFetcher,
    pub blueprint_id: u64,
    pub gadget_name: String,
}

impl BinarySourceFetcher for GithubBinaryFetcher {
    async fn get_binary(&self) -> Result<PathBuf> {
        let relevant_binary =
            get_gadget_binary(&self.fetcher.binaries.0).ok_or(Error::NoMatchingBinary)?;
        let expected_hash = sdk::utils::slice_32_to_sha_hex_string(relevant_binary.sha256);
        let current_dir = std::env::current_dir()?;
        let mut binary_download_path =
            format!("{}/protocol-{:?}", current_dir.display(), self.fetcher.tag);

        if cfg!(target_family = "windows") {
            binary_download_path += ".exe";
        }

        info!("Downloading to {binary_download_path}");

        // Check if the binary exists, if not download it
        if !valid_file_exists(&binary_download_path, &expected_hash).await {
            return Ok(PathBuf::from(binary_download_path));
        }

        let url = get_download_url(relevant_binary, &self.fetcher);

        let download = reqwest::get(&url).await?.bytes().await?;
        let retrieved_hash = hash_bytes_to_hex(&download);

        // Write the binary to disk
        let mut file = tokio::fs::File::create(&binary_download_path).await?;
        file.write_all(&download).await?;
        file.flush().await?;

        if retrieved_hash.trim() != expected_hash.trim() {
            return Err(Error::HashMismatch {
                expected: expected_hash,
                actual: retrieved_hash,
            });
        }

        Ok(PathBuf::from(binary_download_path))
    }

    fn blueprint_id(&self) -> u64 {
        self.blueprint_id
    }

    fn name(&self) -> String {
        self.gadget_name.clone()
    }
}
