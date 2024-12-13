use crate::remote::RemoteConfig;
use std::path::PathBuf;

#[derive(Default)]
pub struct KeystoreConfig {
    pub(crate) in_memory: bool,
    #[cfg(feature = "std")]
    pub(crate) fs_root: Option<PathBuf>,
    #[cfg(any(
        feature = "aws-signer",
        feature = "gcp-signer",
        feature = "ledger-browser",
        feature = "ledger-node"
    ))]
    pub(crate) remote_configs: Vec<RemoteConfig>,
}

impl KeystoreConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn in_memory(mut self, value: bool) -> Self {
        self.in_memory = value;
        self
    }

    #[cfg(feature = "std")]
    pub fn fs_root(mut self, path: PathBuf) -> Self {
        self.fs_root = Some(path);
        self
    }

    cfg_remote! {
        pub fn remote(mut self, remote_config: RemoteConfig) -> Self {
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
