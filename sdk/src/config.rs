use crate::keystore::backend::GenericKeyStore;
#[cfg(any(feature = "std", feature = "wasm"))]
use crate::keystore::BackendExt;
#[cfg(any(feature = "std", feature = "wasm"))]
use crate::keystore::{sp_core_subxt, TanglePairSigner};
use alloc::string::{String, ToString};
use core::fmt::Debug;
use core::net::IpAddr;
use eigensdk::crypto_bls;
use gadget_io::SupportedChains;
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use url::Url;

/// The protocol on which a gadget will be executed.
#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Protocol {
    #[default]
    Tangle,
    Eigenlayer,
}

impl Protocol {
    /// Returns the protocol from the environment variable `PROTOCOL`.
    ///
    /// If the environment variable is not set, it defaults to `Protocol::Tangle`.
    ///
    /// # Errors
    ///
    /// * [`Error::UnsupportedProtocol`] if the protocol is unknown. See [`Protocol`].
    #[cfg(feature = "std")]
    pub fn from_env() -> Result<Self, Error> {
        if let Ok(protocol) = std::env::var("PROTOCOL") {
            return protocol.to_ascii_lowercase().parse::<Protocol>();
        }

        Ok(Protocol::default())
    }

    /// Returns the protocol from the environment variable `PROTOCOL`.
    #[cfg(not(feature = "std"))]
    pub fn from_env() -> Result<Self, Error> {
        Ok(Protocol::default())
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
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tangle" => Ok(Self::Tangle),
            "eigenlayer" => Ok(Self::Eigenlayer),
            _ => Err(Error::UnsupportedProtocol(s.to_string())),
        }
    }
}

/// Gadget environment using the `parking_lot` RwLock.
#[cfg(feature = "std")]
pub type StdGadgetConfiguration = GadgetConfiguration<parking_lot::RawRwLock>;

/// Gadget environment.
#[non_exhaustive]
pub struct GadgetConfiguration<RwLock: lock_api::RawRwLock> {
    /// Tangle RPC endpoint.
    pub rpc_endpoint: String,
    /// Keystore URI
    ///
    /// * In Memory: `file::memory:` or `:memory:`
    /// * Filesystem: `file:/path/to/keystore` or `file:///path/to/keystore`
    pub keystore_uri: String,
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
    /// The type of protocol the gadget is executing on.
    pub protocol: Protocol,
    /// The Port of the Network that will be interacted with
    pub bind_port: u16,
    /// The Address of the Network that will be interacted with
    pub bind_addr: IpAddr,
    pub span: tracing::Span,
    /// Whether the gadget is in test mode
    pub test_mode: bool,
    _lock: core::marker::PhantomData<RwLock>,
}

impl<RwLock: lock_api::RawRwLock> Debug for GadgetConfiguration<RwLock> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GadgetConfiguration")
            .field("rpc_endpoint", &self.rpc_endpoint)
            .field("keystore_uri", &self.keystore_uri)
            .field("blueprint_id", &self.blueprint_id)
            .field("service_id", &self.service_id)
            .field("is_registration", &self.is_registration)
            .field("protocol", &self.protocol)
            .field("bind_port", &self.bind_port)
            .field("bind_addr", &self.bind_addr)
            .field("test_mode", &self.test_mode)
            .finish()
    }
}

impl<RwLock: lock_api::RawRwLock> Clone for GadgetConfiguration<RwLock> {
    fn clone(&self) -> Self {
        Self {
            rpc_endpoint: self.rpc_endpoint.clone(),
            keystore_uri: self.keystore_uri.clone(),
            blueprint_id: self.blueprint_id,
            service_id: self.service_id,
            is_registration: self.is_registration,
            protocol: self.protocol,
            bind_port: self.bind_port,
            bind_addr: self.bind_addr,
            span: self.span.clone(),
            test_mode: self.test_mode,
            _lock: core::marker::PhantomData,
        }
    }
}

// Useful for quick testing
impl<RwLock: lock_api::RawRwLock> Default for GadgetConfiguration<RwLock> {
    fn default() -> Self {
        Self {
            rpc_endpoint: "http://localhost:9944".to_string(),
            keystore_uri: "file::memory:".to_string(),
            blueprint_id: 0,
            service_id: Some(0),
            is_registration: false,
            protocol: Protocol::Tangle,
            bind_port: 0,
            bind_addr: core::net::IpAddr::V4(core::net::Ipv4Addr::new(127, 0, 0, 1)),
            span: tracing::Span::current(),
            test_mode: true,
            _lock: core::marker::PhantomData,
        }
    }
}

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
    /// Missing `KEYSTORE_URI` environment
    #[error("Missing keystore URI")]
    TestSetup(String),
}

#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
#[structopt(name = "General CLI Context")]
#[cfg(feature = "std")]
pub struct ContextConfig {
    /// Pass through arguments to another command
    #[structopt(subcommand)]
    pub gadget_core_settings: GadgetCLICoreSettings,
}

#[derive(Debug, Clone, StructOpt, Serialize, Deserialize)]
#[cfg(feature = "std")]
pub enum GadgetCLICoreSettings {
    #[structopt(name = "run")]
    Run {
        #[structopt(long, short = "b", parse(try_from_str), env)]
        bind_addr: IpAddr,
        #[structopt(long, short = "p", env)]
        bind_port: u16,
        #[structopt(long, short = "t", env)]
        test_mode: bool,
        #[structopt(long, short = "l", env)]
        log_id: Option<String>,
        #[structopt(long, short = "u", parse(try_from_str = url::Url::parse), env)]
        #[serde(default = "gadget_io::defaults::rpc_url")]
        url: Url,
        #[structopt(long, parse(try_from_str = <Multiaddr as std::str::FromStr>::from_str), env)]
        #[serde(default)]
        bootnodes: Option<Vec<Multiaddr>>,
        #[structopt(long, short = "d", env)]
        keystore_uri: String,
        #[structopt(
            long,
            default_value,
            possible_values = &[
                "local_testnet",
                "local_mainnet",
                "testnet",
                "mainnet"
            ],
            env
        )]
        chain: SupportedChains,
        #[structopt(long, short = "v", parse(from_occurrences), env)]
        verbose: i32,
        /// Whether to use pretty logging
        #[structopt(long, env)]
        pretty: bool,
        #[structopt(long, env)]
        keystore_password: Option<String>,
        #[structopt(long, env)]
        blueprint_id: u64,
        #[structopt(long, env)]
        service_id: Option<u64>,
        #[structopt(long, parse(try_from_str), env)]
        protocol: Protocol,
    },
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
#[cfg(feature = "std")] // TODO: Add no_std support
pub fn load_with_lock<RwLock: lock_api::RawRwLock>(
    config: ContextConfig,
) -> Result<GadgetConfiguration<RwLock>, Error> {
    load_inner::<RwLock>(config)
}

#[cfg(feature = "std")]
fn load_inner<RwLock: lock_api::RawRwLock>(
    config: ContextConfig,
) -> Result<GadgetConfiguration<RwLock>, Error> {
    let is_registration = std::env::var("REGISTRATION_MODE_ON").is_ok();
    let ContextConfig {
        gadget_core_settings:
            GadgetCLICoreSettings::Run {
                bind_addr,
                bind_port,
                test_mode,
                log_id,
                url,
                keystore_uri,
                blueprint_id,
                service_id,
                protocol,
                ..
            },
        ..
    } = config;

    let span = match log_id {
        Some(id) => tracing::info_span!("gadget", id = id),
        None => tracing::info_span!("gadget"),
    };

    Ok(GadgetConfiguration {
        bind_addr,
        bind_port,
        test_mode,
        span,
        rpc_endpoint: url.to_string(),
        keystore_uri,
        blueprint_id,
        // If the registration mode is on, we don't need the service ID
        service_id: if is_registration {
            None
        } else {
            Some(service_id.ok_or_else(|| Error::MissingServiceId)?)
        },
        is_registration,
        protocol,
        _lock: core::marker::PhantomData,
    })
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
                Ok(GenericKeyStore::Fs(FilesystemKeystore::open(path)?))
            }
            otherwise => Err(Error::UnsupportedKeystoreUri(otherwise.to_string())),
        }
    }

    /// Returns the first Sr25519 signer keypair from the keystore.
    ///
    /// # Errors
    ///
    /// * No sr25519 keypair is found in the keystore.
    /// * The keypair seed is invalid.
    #[doc(alias = "sr25519_signer")]
    #[cfg(any(feature = "std", feature = "wasm"))]
    pub fn first_sr25519_signer(
        &self,
    ) -> Result<TanglePairSigner<sp_core_subxt::sr25519::Pair>, Error> {
        self.keystore()?.sr25519_key().map_err(Error::Keystore)
    }

    /// Returns the first ECDSA signer keypair from the keystore.
    ///
    /// # Errors
    ///
    /// * No ECDSA keypair is found in the keystore.
    /// * The keypair seed is invalid.
    #[doc(alias = "ecdsa_signer")]
    #[cfg(any(feature = "std", feature = "wasm"))]
    pub fn first_ecdsa_signer(
        &self,
    ) -> Result<TanglePairSigner<sp_core_subxt::ecdsa::Pair>, Error> {
        self.keystore()?.ecdsa_key().map_err(Error::Keystore)
    }

    /// Returns the first ED25519 signer keypair from the keystore.
    ///
    /// # Errors
    ///
    /// * No ED25519 keypair is found in the keystore.
    /// * The keypair seed is invalid.
    #[doc(alias = "ed25519_signer")]
    #[cfg(any(feature = "std", feature = "wasm"))]
    pub fn first_ed25519_signer(
        &self,
    ) -> Result<TanglePairSigner<sp_core_subxt::ed25519::Pair>, Error> {
        self.keystore()?.ed25519_key().map_err(Error::Keystore)
    }

    /// Returns the first BLS BN254 signer keypair from the keystore.
    ///
    /// # Errors
    ///
    /// This function will return an error if no BLS BN254 keypair is found in the keystore.
    #[doc(alias = "bls_bn254_signer")]
    #[cfg(any(feature = "std", feature = "wasm"))]
    pub fn first_bls_bn254_signer(&self) -> Result<crypto_bls::BlsKeyPair, Error> {
        self.keystore()?.bls_bn254_key().map_err(Error::Keystore)
    }

    /// Returns whether the gadget should run in registration mode.
    #[must_use]
    pub const fn should_run_registration(&self) -> bool {
        self.is_registration
    }

    /// Returns a new [`subxt::OnlineClient`] for the Tangle.
    ///
    /// # Errors
    /// This function will return an error if we are unable to connect to the Tangle RPC endpoint.
    #[cfg(any(feature = "std", feature = "wasm"))]
    pub async fn client(&self) -> Result<crate::clients::tangle::runtime::TangleClient, Error> {
        let client =
            subxt::OnlineClient::<crate::clients::tangle::runtime::TangleConfig>::from_url(
                self.rpc_endpoint.clone(),
            )
            .await?;
        Ok(client)
    }
}
