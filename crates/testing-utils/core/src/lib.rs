use cargo_toml::Manifest;
pub use error::TestRunnerError;
pub use runner::TestRunner;
use std::path::Path;

mod error;
pub use error::TestRunnerError as Error;

pub mod harness;
pub mod runner;

/// Reads the manifest at `path`
///
/// # Errors
///
/// * The manifest is invalid
/// * The manifest does not have a `package` section
pub fn read_cargo_toml_file<P: AsRef<Path>>(path: P) -> std::io::Result<Manifest> {
    let manifest = cargo_toml::Manifest::from_path(path)
        .map_err(|err| std::io::Error::other(format!("Failed to read Cargo.toml: {err}")))?;
    if manifest.package.is_none() {
        return Err(std::io::Error::other(
            "No package section found in Cargo.toml",
        ));
    }

    Ok(manifest)
}
