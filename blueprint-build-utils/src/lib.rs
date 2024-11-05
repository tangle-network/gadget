use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Build the Smart contracts at the specified directories, automatically rerunning if changes are
/// detected in this crates Smart Contracts (`./contracts/lib`).
///
/// # Panics
/// - If the Cargo Manifest directory is not found.
/// - If the `forge` executable is not found.
pub fn build_contracts(contract_dirs: Vec<&str>) {
    println!("cargo::rerun-if-changed=contracts/lib/*");
    println!("cargo::rerun-if-changed=contracts/src/*");

    // Get the project root directory
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Try to find the `forge` executable dynamically
    let forge_executable = match Command::new("which").arg("forge").output() {
        Ok(output) => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            assert!(
                !path.is_empty(),
                "Forge executable not found. Make sure Foundry is installed."
            );
            path
        }
        Err(e) => panic!("Failed to find `forge` executable: {e}"),
    };

    for dir in contract_dirs {
        let full_path = root.join(dir).canonicalize().unwrap_or_else(|_| {
            println!(
                "Directory not found or inaccessible: {}",
                root.join(dir).display()
            );
            root.join(dir)
        });

        if full_path.exists() {
            println!("cargo:rerun-if-changed={}", full_path.display());

            let status = Command::new(&forge_executable)
                .current_dir(&full_path)
                .arg("build")
                .status()
                .expect("Failed to execute Forge build");

            assert!(
                status.success(),
                "Forge build failed for directory: {}",
                full_path.display()
            );
        } else {
            println!(
                "Directory not found or does not exist: {}",
                full_path.display()
            );
        }
    }
}
