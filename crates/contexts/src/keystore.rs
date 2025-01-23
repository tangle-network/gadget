use gadget_config::GadgetConfiguration;
use gadget_keystore::{Keystore, KeystoreConfig};

/// `KeystoreContext` trait provides access to the generic keystore from the context.
pub trait KeystoreContext {
    /// Get the keystore client from the context.
    fn keystore(&self) -> Keystore;
}

impl KeystoreContext for GadgetConfiguration {
    fn keystore(&self) -> Keystore {
        let config = KeystoreConfig::new().fs_root(self.keystore_uri.clone());
        Keystore::new(config).expect("Failed to create keystore")
    }
}
