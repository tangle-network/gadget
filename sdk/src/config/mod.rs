use alloc::string::{String, ToString};
use core::fmt::Debug;
use std::path::PathBuf;

pub mod context_config;
pub mod gadget_config;
pub mod protocol;

pub use context_config::{ContextConfig, GadgetCLICoreSettings};
pub use gadget_config::{GadgetConfiguration, StdGadgetConfiguration};
pub use protocol::{Protocol, ProtocolSpecificSettings};

/// Errors that can occur while loading and using the gadget configuration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Missing `RPC_URL` environment variable.
    #[error("Missing Tangle RPC endpoint")]
    MissingTangleRpcEndpoint,
    /// Missing `KEYSTORE_URI` environment
    #[error("Missing keystore URI")]
    MissingKeystoreUri,
    /// Missing `BLUEPRINT_ID` environment variable
    #[error("Missing blueprint ID")]
    MissingBlueprintId,
    /// Missing `SERVICE_ID` environment variable
    #[error("Missing service ID")]
    MissingServiceId,
    /// Error parsing the blueprint ID.
    #[error(transparent)]
    MalformedBlueprintId(core::num::ParseIntError),
    /// Error parsing the service ID.
    #[error(transparent)]
    MalformedServiceId(core::num::ParseIntError),
    /// Unsupported keystore URI.
    #[error("Unsupported keystore URI: {0}")]
    UnsupportedKeystoreUri(String),
    /// Error opening the filesystem keystore.
    #[error(transparent)]
    Keystore(#[from] crate::keystore::Error),
    /// Subxt error.
    #[error(transparent)]
    #[cfg(any(feature = "std", feature = "wasm"))]
    Subxt(#[from] subxt::Error),
    /// Error parsing the protocol, from the `PROTOCOL` environment variable.
    #[error("Unsupported protocol: {0}")]
    UnsupportedProtocol(String),
    /// No Sr25519 keypair found in the keystore.
    #[error("No Sr25519 keypair found in the keystore")]
    NoSr25519Keypair,
    /// Invalid Sr25519 keypair found in the keystore.
    #[error("Invalid Sr25519 keypair found in the keystore")]
    InvalidSr25519Keypair,
    /// No ECDSA keypair found in the keystore.
    #[error("No ECDSA keypair found in the keystore")]
    NoEcdsaKeypair,
    /// Invalid ECDSA keypair found in the keystore.
    #[error("Invalid ECDSA keypair found in the keystore")]
    InvalidEcdsaKeypair,
    /// Test setup error
    #[error("Test setup error: {0}")]
    TestSetup(String),
    /// Missing `EigenlayerContractAddresses`
    #[error("Missing EigenlayerContractAddresses")]
    MissingEigenlayerContractAddresses,
    /// Missing `SymbioticContractAddresses`
    #[error("Missing EigenlayerContractAddresses")]
    MissingSymbioticContractAddresses,
    #[error("Bad RPC Connection: {0}")]
    BadRpcConnection(String),
}

/// Loads the [`GadgetConfiguration`] from the current environment.
/// # Errors
///
/// This function will return an error if any of the required environment variables are missing.
#[cfg(feature = "std")]
pub fn load(config: ContextConfig) -> Result<GadgetConfiguration<parking_lot::RawRwLock>, Error> {
    load_with_lock::<parking_lot::RawRwLock>(config)
}

/// Loads the [`GadgetConfiguration`] from the current environment.
///
/// This allows callers to specify the `RwLock` implementation to use.
///
/// # Errors
///
/// This function will return an error if any of the required environment variables are missing.
// TODO: Add no_std support
#[cfg(feature = "std")]
pub fn load_with_lock<RwLock: lock_api::RawRwLock>(
    config: ContextConfig,
) -> Result<GadgetConfiguration<RwLock>, Error> {
    load_inner::<RwLock>(config)
}

#[cfg(feature = "std")]
fn load_inner<RwLock: lock_api::RawRwLock>(
    config: ContextConfig,
) -> Result<GadgetConfiguration<RwLock>, Error> {
    use protocol::{
        EigenlayerContractAddresses, SymbioticContractAddresses, TangleInstanceSettings,
    };

    let is_registration = std::env::var("REGISTRATION_MODE_ON").is_ok();
    let ContextConfig {
        gadget_core_settings:
            GadgetCLICoreSettings::Run {
                target_addr: bind_addr,
                target_port: bind_port,
                use_secure_url,
                test_mode,
                log_id,
                http_rpc_url,
                ws_rpc_url,
                bootnodes,
                keystore_uri,
                protocol,
                blueprint_id,
                service_id,
                skip_registration,
                registry_coordinator,
                operator_state_retriever,
                delegation_manager,
                strategy_manager,
                avs_directory,
                operator_registry,
                network_registry,
                base_delegator,
                network_opt_in_service,
                vault_opt_in_service,
                slasher,
                veto_slasher,
                ..
            },
        ..
    } = config;

    let span = match log_id {
        Some(id) => tracing::info_span!("gadget", id = id),
        None => tracing::info_span!("gadget"),
    };

    let protocol_specific = match protocol {
        Protocol::Eigenlayer => ProtocolSpecificSettings::Eigenlayer(EigenlayerContractAddresses {
            registry_coordinator_address: registry_coordinator
                .ok_or(Error::MissingEigenlayerContractAddresses)?,
            operator_state_retriever_address: operator_state_retriever
                .ok_or(Error::MissingEigenlayerContractAddresses)?,
            delegation_manager_address: delegation_manager
                .ok_or(Error::MissingEigenlayerContractAddresses)?,
            strategy_manager_address: strategy_manager
                .ok_or(Error::MissingEigenlayerContractAddresses)?,
            avs_directory_address: avs_directory
                .ok_or(Error::MissingEigenlayerContractAddresses)?,
        }),
        Protocol::Symbiotic => ProtocolSpecificSettings::Symbiotic(SymbioticContractAddresses {
            operator_registry_address: operator_registry
                .ok_or(Error::MissingSymbioticContractAddresses)?,
            network_registry_address: network_registry
                .ok_or(Error::MissingSymbioticContractAddresses)?,
            base_delegator_address: base_delegator
                .ok_or(Error::MissingSymbioticContractAddresses)?,
            network_opt_in_service_address: network_opt_in_service
                .ok_or(Error::MissingSymbioticContractAddresses)?,
            vault_opt_in_service_address: vault_opt_in_service
                .ok_or(Error::MissingSymbioticContractAddresses)?,
            slasher_address: slasher.ok_or(Error::MissingSymbioticContractAddresses)?,
            veto_slasher_address: veto_slasher.ok_or(Error::MissingSymbioticContractAddresses)?,
        }),
        Protocol::Tangle => ProtocolSpecificSettings::Tangle(TangleInstanceSettings {
            blueprint_id: blueprint_id.ok_or(Error::MissingBlueprintId)?,
            // If we are in registration mode, we don't need a service id
            service_id: if !is_registration {
                Some(service_id.ok_or(Error::MissingServiceId)?)
            } else {
                None
            },
        }),
    };

    Ok(GadgetConfiguration {
        target_addr: bind_addr,
        target_port: bind_port,
        use_secure_url,
        test_mode,
        span,
        http_rpc_endpoint: http_rpc_url.to_string(),
        ws_rpc_endpoint: ws_rpc_url.to_string(),
        keystore_uri,
        data_dir: std::env::var("DATA_DIR").ok().map(PathBuf::from),
        bootnodes: bootnodes.unwrap_or_default(),
        is_registration,
        skip_registration,
        protocol,
        protocol_specific,
        _lock: core::marker::PhantomData,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        ContextConfig::command().debug_assert();
    }
}
