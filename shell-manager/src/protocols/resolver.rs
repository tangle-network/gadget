use crate::error::Error;
use crate::protocols::config::ProtocolConfig;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
pub struct NativeGithubMetadata {
    pub git: String,
    pub tag: String,
    pub bin_hashes: HashMap<String, String>,
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
            let git = repository.get("git").ok_or(Error::msg(
                "External protocol does not have a git repository specified",
            ))?;
            let tag = repository.get("tag").cloned().ok_or(Error::msg(
                "External protocol does not have a revision specified",
            ))?;

            ret.push(NativeGithubMetadata {
                git: git.clone(),
                tag,
                bin_hashes,
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
