//! Keystore Errors

/// Different errors that can occur in the [`crate::keystore`] module
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O error occurred
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
