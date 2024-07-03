use crate::error::Error;
use crate::protocols::config::ProtocolConfig;
use std::collections::HashMap;
use std::path::Path;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::ServiceBlueprint;

#[derive(Debug)]
pub struct ProtocolMetadata {
    pub service: ServiceBlueprint,
    pub git: String,
    pub rev: String,
    pub package: String,
    pub bin_hashes: HashMap<String, String>,
}

pub fn load_global_config_file<P: AsRef<Path>>(path: P) -> Result<Vec<ProtocolMetadata>, Error> {
    let config: ProtocolConfig = toml::from_str(&std::fs::read_to_string(path)?)
        .map_err(|err| Error::msg(err.to_string()))?;
    let mut ret = vec![];

    for protocol in config.protocols {
        if protocol.role_types.is_empty() {
            return Err(Error::msg(
                "Protocol does not have any role types specified",
            ));
        }

        if let (Some(bin_hashes), Some(repository)) = (protocol.bin_hashes, protocol.repository) {
            if bin_hashes.is_empty() {
                return Err(Error::msg(
                    "External protocol does not have any binaries hashes specified",
                ));
            }
            let git = repository.get("git").ok_or(Error::msg(
                "External protocol does not have a git repository specified",
            ))?;
            let rev = repository.get("rev").cloned().ok_or(Error::msg(
                "External protocol does not have a revision specified",
            ))?;
            ret.push(ProtocolMetadata {
                git: git.clone(),
                service: convert_str_vec_to_role_types(protocol.role_types)?,
                rev,
                bin_hashes,
                package: protocol.package,
            })
        } else {
            return Err(Error::msg(format!(
                "External protocol does not have bin_hashes and a repository specified: {:?}",
                protocol.role_types
            )));
        }
    }

    Ok(ret)
}
