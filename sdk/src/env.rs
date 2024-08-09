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
    pub service_id: u64,
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
}

/// Loads the [`GadgetEnvironment`] from the current environment.
/// # Errors
///
/// This function will return an error if any of the required environment variables are missing.
#[cfg(feature = "std")]
pub fn load() -> Result<GadgetEnvironment, Error> {
    Ok(GadgetEnvironment {
        tangle_rpc_endpoint: std::env::var("RPC_URL")
            .map_err(|_| Error::MissingTangleRpcEndpoint)?,
        keystore_uri: std::env::var("KEYSTORE_URI").map_err(|_| Error::MissingKeystoreUri)?,
        data_dir_path: std::env::var("DATA_DIR").ok(),
        blueprint_id: std::env::var("BLUEPRINT_ID")
            .map_err(|_| Error::MissingBlueprintId)?
            .parse()
            .map_err(Error::MalformedBlueprintId)?,
        service_id: std::env::var("SERVICE_ID")
            .map_err(|_| Error::MissingServiceId)?
            .parse()
            .map_err(Error::MalformedServiceId)?,
    })
}

#[cfg(not(feature = "std"))]
pub fn load() -> Result<GadgetEnvironment, Error> {
    unimplemented!("Implement loading env for no_std")
}

impl GadgetEnvironment {
    /// Loads the `KeyStore` from the current environment.
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

    /// Returns whether the gadget should run in memory.
    #[must_use]
    pub const fn should_run_in_memory(&self) -> bool {
        self.data_dir_path.is_none()
    }
}
