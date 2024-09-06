use crate::events_watcher::tangle::TangleConfig;
use crate::keystore::backend::GenericKeyStore;
use alloc::string::{String, ToString};
use core::fmt::Debug;
use gadget_common::prelude::DebugLogger;
use std::net::IpAddr;

#[derive(Default, Debug, Clone, Copy)]
pub enum Protocol {
    #[default]
    Tangle,
    Eigenlayer,
}

impl Protocol {
    /// Returns the protocol from the environment variable `PROTOCOL`.
    #[cfg(feature = "std")]
    pub fn from_env() -> Self {
        std::env::var("PROTOCOL")
            .map(|v| v.parse::<Protocol>().unwrap_or_default())
            .unwrap_or_default()
    }

    /// Returns the protocol from the environment variable `PROTOCOL`.
    #[cfg(not(feature = "std"))]
    pub fn from_env() -> Self {
        Self::Tangle
    }
}

impl core::fmt::Display for Protocol {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::Tangle => write!(f, "tangle"),
            Self::Eigenlayer => write!(f, "eigenlayer"),
        }
    }
}

impl core::str::FromStr for Protocol {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "tangle" => Ok(Self::Tangle),
            "eigenlayer" => Ok(Self::Eigenlayer),
            _ => Err(()),
        }
    }
}

/// Gadget environment using the `parking_lot` RwLock.
#[cfg(feature = "std")]
pub type StdGadgetConfiguration = GadgetConfiguration<parking_lot::RawRwLock>;

/// Gadget environment.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GadgetConfiguration<RwLock: lock_api::RawRwLock> {
    /// RPC endpoint.
    pub rpc_endpoint: String,

    /// Keystore URI
    ///
    /// * In Memory: `file::memory:` or `:memory:`
    /// * Filesystem: `file:/path/to/keystore` or `file:///path/to/keystore`
    pub keystore_uri: String,

    /// Data directory path.
    /// This is used to store the data for the gadget.
    /// If not provided, the gadget is expected to store the data in memory.
    pub data_dir_path: Option<String>,

    /// Blueprint ID for this gadget.
    pub blueprint_id: u64,

    /// Service ID for this gadget.
    ///
    /// This is only set to `None` when the gadget is in the registration mode.
    /// Always check for is `is_registration` flag before using this.
    pub service_id: Option<u64>,

    /// The Current Environment is for the `PreRegisteration` of the Gadget
    ///
    /// The gadget will now start in the Registration mode and will try to register the current operator on that blueprint
    /// There is no Service ID for this mode, since we need to register the operator first on the blueprint.
    ///
    /// If this is set to true, the gadget should do some work and register the operator on the blueprint.
    pub is_registration: bool,

    /// The Current Environment is for the `Benchmarking` of the Gadget
    ///
    /// The gadget will now start in the Benchmarking mode and will try to benchmark the current operator on that blueprint
    /// There is no Service ID for this mode, since we need to benchmark the operator first on the blueprint.
    ///
    /// If this is set to true, the gadget should do some work and benchmark the operator on the blueprint.
    pub is_benchmarking: bool,

    /// The type of protocol the gadget is executing on.
    pub protocol: Protocol,

    /// Bind port
    pub bind_port: u16,

    /// Bind addr
    pub bind_addr: IpAddr,

    /// logger
    pub logger: DebugLogger,

    /// Whether or not the gadget is in test mode
    pub test_mode: bool,

    _lock: core::marker::PhantomData<RwLock>,
}

// impl<RwLock: lock_api::RawRwLock> Debug for GadgetConfiguration<RwLock> {
//     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//         f.debug_struct("GadgetConfiguration")
//             .field("rpc_endpoint", &self.rpc_endpoint)
//             .field("keystore_uri", &self.keystore_uri)
//             .field("data_dir_path", &self.data_dir_path)
//             .field("blueprint_id", &self.blueprint_id)
//             .field("service_id", &self.service_id)
//             .field("is_registration", &self.is_registration)
//             .field("protocol", &self.protocol)
//             .field("bind_port", &self.bind_port)
//             .field("bind_addr", &self.bind_addr)
//             .field("logger", &self.logger)
//             .field("test_mode", &self.test_mode)
//             .finish()
//     }
// }
//
// impl<RwLock: lock_api::RawRwLock> Clone for GadgetConfiguration<RwLock> {
//     fn clone(&self) -> Self {
//         Self {
//             rpc_endpoint: self.rpc_endpoint.clone(),
//             keystore_uri: self.keystore_uri.clone(),
//             data_dir_path: self.data_dir_path.clone(),
//             blueprint_id: self.blueprint_id,
//             service_id: self.service_id,
//             is_registration: self.is_registration,
//             protocol: self.protocol,
//             bind_port: self.bind_port,
//             bind_addr: self.bind_addr,
//             logger: self.logger.clone(),
//             test_mode: self.test_mode,
//             _lock: core::marker::PhantomData,
//         }
//     }
// }

/// An error type for the gadget environment.
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
    Subxt(#[from] subxt::Error),

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
    /// Missing `KEYSTORE_URI` environment
    #[error("Missing keystore URI")]
    TestSetup(String),
}

#[derive(Clone, Debug)]
pub struct ContextConfig {
    pub bind_addr: IpAddr,
    pub bind_port: u16,
    pub test_mode: bool,
    pub logger: DebugLogger,
}

impl ContextConfig {
    pub fn new_test(bind_addr: IpAddr, logger: DebugLogger) -> Result<Self, Error> {
        let unused_port = std::net::TcpListener::bind(format!("{bind_addr}:0"))
            .map_err(|err| Error::TestSetup(err.to_string()))?
            .local_addr()
            .map_err(|err| Error::TestSetup(err.to_string()))?
            .port();
        Ok(Self {
            bind_addr,
            bind_port: unused_port,
            test_mode: true,
            logger,
        })
    }
}

/// Loads the [`GadgetConfiguration`] from the current environment.
/// # Errors
///
/// This function will return an error if any of the required environment variables are missing.
// TODO: ensure that this function takes-in the bind addr, bind port, and test_mode status
#[cfg(feature = "std")]
pub fn load(
    protocol: Option<Protocol>,
    context_config: ContextConfig,
) -> Result<GadgetConfiguration<parking_lot::RawRwLock>, Error> {
    load_with_lock::<parking_lot::RawRwLock>(protocol, context_config)
}

/// Loads the [`GadgetConfiguration`] from the current environment.
///
/// This allows callers to specify the `RwLock` implementation to use.
///
/// # Errors
///
/// This function will return an error if any of the required environment variables are missing.
pub fn load_with_lock<RwLock: lock_api::RawRwLock>(
    protocol: Option<Protocol>,
    context_config: ContextConfig,
) -> Result<GadgetConfiguration<RwLock>, Error> {
    load_inner::<RwLock>(protocol, context_config)
}

#[cfg(feature = "std")]
fn load_inner<RwLock: lock_api::RawRwLock>(
    protocol: Option<Protocol>,
    context_config: ContextConfig,
) -> Result<GadgetConfiguration<RwLock>, Error> {
    let is_registration = std::env::var("REGISTRATION_MODE_ON").is_ok();
    let is_benchmarking = std::env::var("BENCHMARKING_MODE_ON").is_ok();
    Ok(GadgetConfiguration {
        bind_addr: context_config.bind_addr,
        bind_port: context_config.bind_port,
        test_mode: context_config.test_mode,
        logger: context_config.logger,
        rpc_endpoint: std::env::var("RPC_URL").map_err(|_| Error::MissingTangleRpcEndpoint)?,
        keystore_uri: std::env::var("KEYSTORE_URI").map_err(|_| Error::MissingKeystoreUri)?,
        data_dir_path: std::env::var("DATA_DIR").ok(),
        blueprint_id: std::env::var("BLUEPRINT_ID")
            .map_err(|_| Error::MissingBlueprintId)?
            .parse()
            .map_err(Error::MalformedBlueprintId)?,
        // If the registration mode is on, or benchmarking mode, we don't need the service ID
        service_id: if is_registration || is_benchmarking {
            None
        } else {
            Some(
                std::env::var("SERVICE_ID")
                    .map_err(|_| Error::MissingServiceId)?
                    .parse()
                    .map_err(Error::MalformedServiceId)?,
            )
        },
        is_registration,
        is_benchmarking,
        protocol: protocol.unwrap_or(Protocol::Tangle),
        _lock: core::marker::PhantomData,
    })
}

#[cfg(not(feature = "std"))]
pub fn load_inner<RwLock: lock_api::RawRwLock>() -> Result<GadgetConfiguration<RwLock>, Error> {
    unimplemented!("Implement loading env for no_std")
}

impl<RwLock: lock_api::RawRwLock> GadgetConfiguration<RwLock> {
    /// Loads the `KeyStore` from the current environment.
    ///
    /// # Errors
    ///
    /// This function will return an error if the keystore URI is unsupported.
    pub fn keystore(&self) -> Result<GenericKeyStore<RwLock>, Error> {
        #[cfg(feature = "std")]
        use crate::keystore::backend::fs::FilesystemKeystore;
        use crate::keystore::backend::{mem::InMemoryKeystore, GenericKeyStore};

        match self.keystore_uri.as_str() {
            uri if uri == "file::memory:" || uri == ":memory:" => {
                Ok(GenericKeyStore::Mem(InMemoryKeystore::new()))
            }
            #[cfg(feature = "std")]
            uri if uri.starts_with("file:") || uri.starts_with("file://") => {
                let path = uri
                    .trim_start_matches("file://")
                    .trim_start_matches("file:");
                tracing::debug!("Reading keystore from: {path}");
                Ok(GenericKeyStore::Fs(FilesystemKeystore::open(path)?))
            }
            otherwise => Err(Error::UnsupportedKeystoreUri(otherwise.to_string())),
        }
    }

    /// Returns the first Sr25519 signer keypair from the keystore.
    ///
    /// # Errors
    ///
    /// This function will return an error if no Sr25519 keypair is found in the keystore.
    /// or if the keypair seed is invalid.
    #[doc(alias = "sr25519_signer")]
    pub fn first_signer(&self) -> Result<tangle_subxt::subxt_signer::sr25519::Keypair, Error> {
        let keystore = self.keystore()?;
        let sr25519_pubkey = crate::keystore::Backend::iter_sr25519(&keystore)
            .next()
            .ok_or_else(|| Error::NoSr25519Keypair)?;
        let sr25519_secret =
            crate::keystore::Backend::expose_sr25519_secret(&keystore, &sr25519_pubkey)?
                .ok_or_else(|| Error::NoSr25519Keypair)?;

        let mut seed = [0u8; 32];
        seed.copy_from_slice(&sr25519_secret.to_bytes()[0..32]);
        tangle_subxt::subxt_signer::sr25519::Keypair::from_secret_key(seed)
            .map_err(|_| Error::InvalidSr25519Keypair)
    }

    /// Returns the first ECDSA signer keypair from the keystore.
    ///
    /// # Errors
    ///
    /// This function will return an error if no ECDSA keypair is found in the keystore.
    /// or if the keypair seed is invalid.
    #[doc(alias = "ecdsa_signer")]
    pub fn first_ecdsa_signer(&self) -> Result<tangle_subxt::subxt_signer::ecdsa::Keypair, Error> {
        let keystore = self.keystore()?;
        let ecdsa_pubkey = crate::keystore::Backend::iter_ecdsa(&keystore)
            .next()
            .ok_or_else(|| Error::NoEcdsaKeypair)?;
        let ecdsa_secret = crate::keystore::Backend::expose_ecdsa_secret(&keystore, &ecdsa_pubkey)?
            .ok_or_else(|| Error::NoEcdsaKeypair)?;

        let mut seed = [0u8; 32];
        seed.copy_from_slice(&ecdsa_secret.to_bytes()[0..32]);
        tangle_subxt::subxt_signer::ecdsa::Keypair::from_secret_key(seed)
            .map_err(|_| Error::InvalidEcdsaKeypair)
    }

    /// Returns the first BLS BN254 signer keypair from the keystore.
    ///
    /// # Errors
    ///
    /// This function will return an error if no ECDSA keypair is found in the keystore.
    /// or if the keypair seed is invalid.
    #[doc(alias = "bls_bn254_signer")]
    pub fn first_bls_bn254_signer(
        &self,
    ) -> Result<eigensdk_rs::eigen_utils::crypto::bls::KeyPair, Error> {
        let keystore = self.keystore()?;
        let bls_pubkey = crate::keystore::Backend::iter_bls_bn254(&keystore)
            .next()
            .ok_or_else(|| Error::NoEcdsaKeypair)?;
        let bls_secret = crate::keystore::Backend::expose_bls_bn254_secret(&keystore, &bls_pubkey)?
            .ok_or_else(|| Error::NoEcdsaKeypair)?;
        Ok(eigensdk_rs::eigen_utils::crypto::bls::KeyPair::new(
            bls_secret,
        ))
    }

    /// Returns whether the gadget should run in memory.
    #[must_use]
    pub const fn should_run_in_memory(&self) -> bool {
        self.data_dir_path.is_none()
    }

    /// Returns whether the gadget should run in registration mode.
    #[must_use]
    pub const fn should_run_registration(&self) -> bool {
        self.is_registration
    }

    /// Returns whether the gadget should run in benchmarking mode.
    #[must_use]
    pub const fn should_run_benchmarks(&self) -> bool {
        self.is_benchmarking
    }

    /// Returns a new [`subxt::OnlineClient`] for the Tangle.
    ///
    /// # Errors
    /// This function will return an error if we are unable to connect to the Tangle RPC endpoint.
    pub async fn client(&self) -> Result<subxt::OnlineClient<TangleConfig>, Error> {
        let client =
            subxt::OnlineClient::<TangleConfig>::from_url(self.rpc_endpoint.clone()).await?;
        Ok(client)
    }
}
