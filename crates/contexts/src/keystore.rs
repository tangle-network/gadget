use gadget_keystore::Keystore;

/// `KeystoreContext` trait provides access to the generic keystore from the context.
pub trait KeystoreContext {
    /// Get the keystore client from the context.
    fn keystore(&self) -> Keystore;
}
