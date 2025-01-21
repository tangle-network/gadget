use cargo_toml::Manifest;
pub use error::TestRunnerError;
use gadget_config::{ContextConfig, GadgetCLICoreSettings};
use gadget_logging::info;
pub use runner::TestRunner;
use std::io::Write;
use std::path::Path;

mod error;
pub use error::TestRunnerError as Error;

pub mod harness;
pub mod runner;

pub fn read_cargo_toml_file<P: AsRef<Path>>(path: P) -> std::io::Result<Manifest> {
    let manifest = cargo_toml::Manifest::from_path(path).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to read Cargo.toml: {err}"),
        )
    })?;
    if manifest.package.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "No package section found in Cargo.toml",
        ));
    }

    Ok(manifest)
}

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
