use gadget_keystore::Keystore;

/// `KeystoreContext` trait provides access to the generic keystore from the context.
pub trait KeystoreContext {
    /// Get the keystore client from the context.
    fn keystore(&self) -> Keystore;
}

#[cfg(feature = "std")]
impl KeystoreContext for blueprint_runner::config::BlueprintEnvironment {
    fn keystore(&self) -> Keystore {
        let config = gadget_keystore::KeystoreConfig::new().fs_root(self.keystore_uri.clone());
        Keystore::new(config).expect("Failed to create keystore")
    }
}
