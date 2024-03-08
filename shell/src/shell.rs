use crate::keystore::KeystoreContainer;
use sp_core::crypto::KeyTypeId;
use sp_keystore::Keystore;

const KEY_TYPE: KeyTypeId = KeyTypeId(*b"test");

/// Start the shell and run it forever
#[tracing::instrument(skip(config))]
pub async fn run_forever(config: crate::config::ShellConfig) -> color_eyre::Result<()> {
    let keystore_container = KeystoreContainer::new(&config.keystore)?;
    let keystore = keystore_container.local_keystore();
    tracing::debug!(
        path = %config.keystore.path().and_then(|v| v.canonicalize().ok()).map(|v| v.display().to_string()).unwrap_or("<memory>".into()),
        "Loaded keystore from path"
    );
    let keys = keystore.ecdsa_public_keys(KEY_TYPE);
    if keys.is_empty() || keys.len() > 1 {
        color_eyre::eyre::bail!("Expected exactly one key in keystore, found {}", keys.len());
    }
    let key = keys[0];
    tracing::debug!(%key, "Loaded key from keystore");
    Ok(())
}
