/// The config for a [`Keystore`]
///
/// Depending on the features enabled, this provides methods to enable different storage backends.
///
/// ### Implicit registration
///
/// When no other backends are enabled, an [`InMemoryStorage`] will implicitly be registered by [`Keystore::new()`].
///
/// This means that:
///
/// ```rust
/// use gadget_keystore::{Keystore, KeystoreConfig};
///
/// # fn main() -> gadget_keystore::Result<()> {
/// let config = KeystoreConfig::new().in_memory(true);
/// let keystore = Keystore::new(config)?;
/// # Ok(()) }
/// ```
///
/// is equivalent to:
///
/// ```rust
/// use gadget_keystore::{Keystore, KeystoreConfig};
///
/// # fn main() -> gadget_keystore::Result<()> {
/// let keystore = Keystore::new(KeystoreConfig::new())?;
/// # Ok(()) }
/// ```
///
/// [`InMemoryStorage`]: crate::storage::InMemoryStorage
/// [`Keystore`]: crate::Keystore
/// [`Keystore::new()`]: crate::Keystore::new
#[derive(Default, Debug)]
pub struct KeystoreConfig {
    pub(crate) in_memory: bool,
    #[cfg(feature = "std")]
    pub(crate) fs_root: Option<std::path::PathBuf>,
    #[cfg(any(
        feature = "aws-signer",
        feature = "gcp-signer",
        feature = "ledger-browser",
        feature = "ledger-node"
    ))]
    pub(crate) remote_configs: Vec<crate::remote::RemoteConfig>,
}

impl KeystoreConfig {
    /// Create a new `KeystoreConfig`
    ///
    /// Alias for [`KeystoreConfig::default()`]
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gadget_keystore::{Keystore, KeystoreConfig};
    ///
    /// # fn main() -> gadget_keystore::Result<()> {
    /// let config = KeystoreConfig::new();
    /// let keystore = Keystore::new(config)?;
    /// # Ok(()) }
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an [`InMemoryStorage`] backend
    ///
    /// NOTE: This will be enabled by default if no other storage backends are enabled.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gadget_keystore::{Keystore, KeystoreConfig};
    ///
    /// # fn main() -> gadget_keystore::Result<()> {
    /// let config = KeystoreConfig::new().in_memory(true);
    /// let keystore = Keystore::new(config)?;
    /// # Ok(()) }
    /// ```
    ///
    /// [`InMemoryStorage`]: crate::storage::InMemoryStorage
    pub fn in_memory(mut self, value: bool) -> Self {
        self.in_memory = value;
        self
    }

    /// Register a [`FileStorage`] backend
    ///
    /// See [`FileStorage::new()`] for notes on how `path` is used.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use gadget_keystore::{Keystore, KeystoreConfig};
    ///
    /// # fn main() -> gadget_keystore::Result<()> {
    /// let config = KeystoreConfig::new().fs_root("path/to/keystore");
    /// let keystore = Keystore::new(config)?;
    /// # Ok(()) }
    /// ```
    ///
    /// [`FileStorage`]: crate::storage::FileStorage
    /// [`FileStorage::new()`]: crate::storage::FileStorage::new
    #[cfg(feature = "std")]
    pub fn fs_root<P: AsRef<std::path::Path>>(mut self, path: P) -> Self {
        self.fs_root = Some(path.as_ref().to_path_buf());
        self
    }

    cfg_remote! {
        /// Register a remote backend
        ///
        /// See [`RemoteConfig`] for available options.
        ///
        /// # Examples
        ///
        /// ```rust,no_run
        /// use gadget_keystore::{Keystore, KeystoreConfig};
        /// use gadget_keystore::remote::RemoteConfig;
        ///
        /// # fn main() -> gadget_keystore::Result<()> {
        /// let remote = RemoteConfig::Aws {
        ///     keys: vec![]
        /// };
        ///
        /// let config = KeystoreConfig::new().remote(remote);
        /// let keystore = Keystore::new(config)?;
        /// # Ok(()) }
        /// ```
        ///
        /// [`RemoteConfig`]: crate::remote::RemoteConfig
        pub fn remote(mut self, remote_config: crate::remote::RemoteConfig) -> Self {
            self.remote_configs.push(remote_config);
            self
        }
    }

    fn is_empty(&self) -> bool {
        let mut is_empty = self.in_memory;
        #[cfg(feature = "std")]
        {
            is_empty |= self.fs_root.is_none();
        }
        #[cfg(any(
            feature = "aws-signer",
            feature = "gcp-signer",
            feature = "ledger-browser",
            feature = "ledger-node"
        ))]
        {
            is_empty |= !self.remote_configs.is_empty();
        }

        is_empty
    }

    pub(crate) fn finalize(self) -> Self {
        if self.is_empty() {
            return self.in_memory(true);
        }

        self
    }
}
