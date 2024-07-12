use crate::error::Error;
use crate::protocols::config::ProtocolConfig;
use std::collections::HashMap;
use std::path::Path;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::GadgetBinary;

#[derive(Debug)]
pub struct NativeGithubMetadata {
    pub git: String,
    pub tag: String,
    pub owner: String,
    pub repo: String,
    pub gadget_binaries: Vec<GadgetBinary>,
}

// This is for testing only
pub fn load_global_config_file<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<NativeGithubMetadata>, Error> {
    let config: ProtocolConfig = toml::from_str(&std::fs::read_to_string(path)?)
        .map_err(|err| Error::msg(err.to_string()))?;
    let mut ret = vec![];

    for protocol in config.protocols {
        if let (Some(bin_hashes), Some(repository)) = (protocol.bin_hashes, protocol.repository) {
            if bin_hashes.is_empty() {
                return Err(Error::msg(
                    "External protocol does not have any binaries hashes specified",
                ));
            }

            let tag = repository.get("tag").cloned().ok_or(Error::msg(
                "External protocol does not have a revision specified",
            ))?;

            let repo = repository.get("repo").cloned().ok_or(Error::msg(
                "External protocol does not have a repository specified",
            ))?;

            let owner = repository.get("owner").cloned().ok_or(Error::msg(
                "External protocol does not have an owner specified",
            ))?;

            let git = format!("https://github.com/{owner}/{repo}");

            // TOOD: Use real value
            let gadget_binaries = vec![];

            ret.push(NativeGithubMetadata {
                git,
                repo,
                owner,
                tag,
                gadget_binaries,
            })
        } else {
            return Err(Error::msg(format!(
                "External protocol does not have bin_hashes and a repository specified: {:?}",
                protocol.package
            )));
        }
    }

    Ok(ret)
}
