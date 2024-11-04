use crate::clients::tangle::runtime::TangleClient;
use crate::config::{ContextConfig, GadgetCLICoreSettings};
use crate::info;
use crate::keystore::KeystoreUriSanitizer;
use futures::future::select_ok;
use std::error::Error;
use std::io::Write;

#[allow(irrefutable_let_patterns)]
pub fn check_for_test(config: &ContextConfig) -> Result<(), crate::Error> {
    // create a file to denote we have started
    if let GadgetCLICoreSettings::Run {
        keystore_uri: base_path,
        test_mode,
        ..
    } = &config.gadget_core_settings
    {
        if !*test_mode {
            return Ok(());
        }
        let path = base_path.sanitize_file_path().join("test_started.tmp");
        let mut file = std::fs::File::create(&path).map_err(crate::Error::IoError)?;
        file.write_all(b"test_started")
            .map_err(crate::Error::IoError)?;
        info!("Successfully wrote test file to {}", path.display())
    }

    Ok(())
}

/// Returns either client. Allows flexibility in choosing between HTTP and WS clients
/// depending on the local setup
pub async fn get_client(ws_url: &str, http_url: &str) -> Result<TangleClient, Box<dyn Error>> {
    let task0 = TangleClient::from_url(ws_url);
    let task1 = TangleClient::from_url(http_url);
    Ok(select_ok([Box::pin(task0), Box::pin(task1)]).await?.0)
}
