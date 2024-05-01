use crate::error::Error;
use crate::protocols::config::ProtocolConfig;
use std::collections::HashMap;
use std::path::Path;
use tangle_subxt::tangle_testnet_runtime::api::jobs::events::job_submitted::RoleType;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::roles::tss::ThresholdSignatureRoleType;
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::roles::zksaas::ZeroKnowledgeRoleType;

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
    let config: ProtocolConfig = toml::from_str(&std::fs::read_to_string(path)?)
        .map_err(|err| Error::msg(err.to_string()))?;
    let mut ret = vec![];

    for protocol in config.protocols {
        if protocol.role_types.is_empty() {
            return Err(Error::msg(
                "Protocol does not have any role types specified",
            ));
        }

        if protocol.internal {
            ret.push(ProtocolMetadata::Internal {
                role_types: convert_str_vec_to_role_types(protocol.role_types)?,
            })
        } else if let (Some(bin_hashes), Some(repository)) =
            (protocol.bin_hashes, protocol.repository)
        {
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
            ret.push(ProtocolMetadata::External {
                git: git.clone(),
                role_types: convert_str_vec_to_role_types(protocol.role_types)?,
                rev,
                bin_hashes,
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

pub fn convert_str_vec_to_role_types(input: Vec<String>) -> Result<Vec<RoleType>, Error> {
    let mut ret = vec![];
    for role in input {
        ret.push(
            str_to_role_type(&role)
                .ok_or_else(|| Error::msg(format!("Unregistered RoleType: {role:?}")))?,
        );
    }

    Ok(ret)
}

pub fn str_to_role_type<T: AsRef<str>>(role_type: T) -> Option<RoleType> {
    let role_type = role_type.as_ref();
    match role_type {
        // ZkSaaS types
        "ZkSaaSGroth16" => Some(RoleType::ZkSaaS(ZeroKnowledgeRoleType::ZkSaaSGroth16)),
        "ZkSaaSMarlin" => Some(RoleType::ZkSaaS(ZeroKnowledgeRoleType::ZkSaaSMarlin)),

        // DFNS
        "DfnsCGGMP21Secp256k1" => Some(RoleType::Tss(
            ThresholdSignatureRoleType::DfnsCGGMP21Secp256k1,
        )),
        "DfnsCGGMP21Secp256r1" => Some(RoleType::Tss(
            ThresholdSignatureRoleType::DfnsCGGMP21Secp256r1,
        )),
        "DfnsCGGMP21Stark" => Some(RoleType::Tss(ThresholdSignatureRoleType::DfnsCGGMP21Stark)),

        // Silent Shard
        "SilentShardDKLS23Secp256k1" => Some(RoleType::Tss(
            ThresholdSignatureRoleType::SilentShardDKLS23Secp256k1,
        )),

        // ZcashFrost
        "ZcashFrostP256" => Some(RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostP256)),
        "ZcashFrostP384" => Some(RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostP384)),
        "ZcashFrostSecp256k1" => Some(RoleType::Tss(
            ThresholdSignatureRoleType::ZcashFrostSecp256k1,
        )),
        "ZcashFrostRistretto255" => Some(RoleType::Tss(
            ThresholdSignatureRoleType::ZcashFrostRistretto255,
        )),
        "ZcashFrostEd25519" => Some(RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostEd25519)),
        "ZcashFrostEd448" => Some(RoleType::Tss(ThresholdSignatureRoleType::ZcashFrostEd448)),

        // BLS
        "GennaroDKGBls381" => Some(RoleType::Tss(ThresholdSignatureRoleType::GennaroDKGBls381)),

        // WSTS
        "WstsV2" => Some(RoleType::Tss(ThresholdSignatureRoleType::WstsV2)),

        // Relaying
        "LightClientRelaying" => Some(RoleType::LightClientRelaying),

        // Faulty types
        _ => None,
    }
}
