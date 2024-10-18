use crate::config::{ContextConfig, GadgetCLICoreSettings};
use crate::info;
use crate::keystore::KeystoreUriSanitizer;
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
