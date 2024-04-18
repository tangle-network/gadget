use crate::error::Error;
use crate::protocols::config::ProtocolConfig;
use std::collections::HashMap;
use std::path::Path;
use tangle_subxt::tangle_mainnet_runtime::api::jobs::events::job_submitted::RoleType;

#[derive(Debug)]
pub enum ProtocolMetadata {
    Internal {
        role_types: Vec<RoleType>,
    },
    External {
        role_types: Vec<RoleType>,
        git: String,
        rev: String,
        bin_hashes: HashMap<String, String>,
    },
}

impl ProtocolMetadata {
    pub fn role_types(&self) -> &Vec<RoleType> {
        match self {
            ProtocolMetadata::Internal { role_types } => role_types,
            ProtocolMetadata::External { role_types, .. } => role_types,
        }
    }
}

pub fn load_global_config_file<P: AsRef<Path>>(path: P) -> Result<Vec<ProtocolMetadata>, Error> {
    let config: ProtocolConfig = toml::from_str(&std::fs::read_to_string(path)?)?;
    let mut ret = vec![];

    for protocol in config.protocols {
        if protocol.role_types.is_empty() {
            return Err(Error::from(
                "Protocol does not have any role types specified",
            ));
        }

        if protocol.internal {
            ret.push(ProtocolMetadata::Internal {
                role_types: protocol.role_types,
            })
        } else {
            if let (Some(bin_hashes), Some(repository)) = (protocol.bin_hashes, protocol.repository)
            {
                if bin_hashes.is_empty() {
                    return Err(Error::from(
                        "External protocol does not have any binaries hashes specified",
                    ));
                }
                let git = repository.get("git").ok_or(Error::from(
                    "External protocol does not have a git repository specified",
                ))?;
                let rev = repository.get("rev").cloned().ok_or(Error::from(
                    "External protocol does not have a revision specified",
                ))?;
                ret.push(ProtocolMetadata::External {
                    git: git.clone(),
                    role_types: protocol.role_types,
                    rev,
                    bin_hashes,
                })
            } else {
                return Err(Error::from(format!(
                    "External protocol does not have bin_hashes and a repository specified: {:?}",
                    protocol.role_types
                )));
            }
        }
    }

    Ok(ret)
}
