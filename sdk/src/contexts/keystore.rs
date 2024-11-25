use crate::keystore::backend::GenericKeyStore;

/// `KeystoreContext` trait provides access to the generic keystore from the context.
pub trait KeystoreContext<RwLock: lock_api::RawRwLock> {
    /// Get the keystore client from the context.
    fn keystore(&self) -> color_eyre::Result<GenericKeyStore<RwLock>, crate::config::Error>;
}
