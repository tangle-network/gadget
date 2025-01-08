use gadget_config::{ContextConfig, GadgetCLICoreSettings};
use gadget_logging::info;
use std::io::Write;
use std::path::Path;

pub use gadget_core_testing_utils::Error;

pub mod keys;
pub mod node;
pub mod runner;

#[allow(irrefutable_let_patterns)]
pub fn check_for_test(config: &ContextConfig) -> Result<(), Error> {
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
        let path = Path::new(base_path).join("test_started.tmp");
        let mut file = std::fs::File::create(&path)?;
        file.write_all(b"test_started")?;
        info!("Successfully wrote test file to {}", path.display())
    }

    Ok(())
}
