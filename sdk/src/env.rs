use crate::events_watcher::tangle::TangleConfig;

/// Gadget environment.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GadgetEnvironment {
    /// Tangle RPC endpoint.
    pub tangle_rpc_endpoint: String,
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

    /// The Current Environment is for the PreRegisteration of the Gadget
    ///
    /// The gadget will now start in the Registration mode and will try to register the current operator on that blueprint
    /// There is no Service ID for this mode, since we need to register the operator first on the blueprint.
    ///
    /// If this is set to true, the gadget should do some work and register the operator on the blueprint.
    pub is_registration: bool,
}

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
}

/// Loads the [`GadgetEnvironment`] from the current environment.
/// # Errors
///
/// This function will return an error if any of the required environment variables are missing.
#[cfg(feature = "std")]
pub fn load() -> Result<GadgetEnvironment, Error> {
    let is_registration = std::env::var("REGISTRATION_MODE_ON").is_ok();
    Ok(GadgetEnvironment {
        tangle_rpc_endpoint: std::env::var("RPC_URL")
            .map_err(|_| Error::MissingTangleRpcEndpoint)?,
        keystore_uri: std::env::var("KEYSTORE_URI").map_err(|_| Error::MissingKeystoreUri)?,
        data_dir_path: std::env::var("DATA_DIR").ok(),
        blueprint_id: std::env::var("BLUEPRINT_ID")
            .map_err(|_| Error::MissingBlueprintId)?
            .parse()
            .map_err(Error::MalformedBlueprintId)?,
        // If the registration mode is on, we don't need the service ID
        service_id: if is_registration {
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
    })
}

#[cfg(not(feature = "std"))]
pub fn load() -> Result<GadgetEnvironment, Error> {
    unimplemented!("Implement loading env for no_std")
}

impl GadgetEnvironment {
    /// Loads the `KeyStore` from the current environment.
    ///
    /// # Errors
    ///
    /// This function will return an error if the keystore URI is unsupported.
    pub fn keystore(&self) -> Result<crate::keystore::backend::GenericKeyStore, Error> {
        use crate::keystore::backend::fs::FilesystemKeystore;
        use crate::keystore::backend::{mem::InMemoryKeystore, GenericKeyStore};

        match self.keystore_uri.as_str() {
            uri if uri == "file::memory:" || uri == ":memory:" => {
                Ok(GenericKeyStore::Mem(InMemoryKeystore::new()))
            }
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

    /// Returns a new OnlineClient for the Tangle.
    ///
    /// # Errors
    /// This function will return an error if we are unable to connect to the Tangle RPC endpoint.
    pub async fn client(&self) -> Result<subxt::OnlineClient<TangleConfig>, Error> {
        let client =
            subxt::OnlineClient::<TangleConfig>::from_url(self.tangle_rpc_endpoint.clone()).await?;
        Ok(client)
    }
}
