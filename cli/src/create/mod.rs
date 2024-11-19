pub use crate::create::error::Error;
pub use crate::create::source::Source;
pub use crate::create::types::BlueprintType;
use std::path::Path;
use types::*;

pub mod error;
pub mod source;
pub mod types;

pub fn new_blueprint(
    name: String,
    source: Option<Source>,
    blueprint_type: Option<BlueprintType>,
) -> Result<(), Error> {
    println!("Generating blueprint with name: {}", name);

    let source = source.unwrap_or_default();
    let blueprint_variant = blueprint_type
        .clone()
        .map(|t| t.get_type())
        .unwrap_or_default();
    let template_path_opt: Option<cargo_generate::TemplatePath> = source.into();

    let template_path = template_path_opt.unwrap_or_else(|| {
        // TODO: Interactive selection (#352)
        let template_repo: String = match blueprint_variant {
            Some(BlueprintVariant::Tangle) | None => {
                "https://github.com/tangle-network/blueprint-template".into()
            }
            Some(BlueprintVariant::Eigenlayer(EigenlayerVariant::BLS)) => {
                "https://github.com/tangle-network/eigenlayer-bls-template".into()
            }
            Some(BlueprintVariant::Eigenlayer(EigenlayerVariant::ECDSA)) => {
                "https://github.com/tangle-network/eigenlayer-ecdsa-template".into()
            }
        };

        cargo_generate::TemplatePath {
            git: Some(template_repo),
            branch: Some(String::from("main")),
            ..Default::default()
        }
    });

    let path = cargo_generate::generate(cargo_generate::GenerateArgs {
        template_path,
        list_favorites: false,
        name: Some(name.to_string()),
        force: false,
        verbose: false,
        template_values_file: None,
        silent: false,
        config: None,
        vcs: Some(cargo_generate::Vcs::Git),
        lib: false,
        bin: true,
        ssh_identity: None,
        define: Default::default(),
        init: false,
        destination: None,
        force_git_init: false,
        allow_commands: false,
        overwrite: false,
        skip_submodules: false,
        other_args: Default::default(),
    })
    .map_err(Error::GenerationFailed)?;

    println!("Blueprint generated at: {}", path.display());

    // TODO: Hack, we have to initialize submodules ourselves, cargo-generate just copies
    //       them as normal directories: https://github.com/cargo-generate/cargo-generate/issues/1317
    std::env::set_current_dir(path)?;
    remove_lib_directories()?;

    add_git_submodules(blueprint_type)?;

    Ok(())
}

fn add_git_submodules(blueprint_type: Option<BlueprintType>) -> Result<(), Error> {
    let blueprint_type = blueprint_type.unwrap_or_default();
    let submodules = blueprint_type.get_submodules();

    for (repo_url, submodule_name) in submodules {
        let submodule_path = format!("contracts/lib/{}", submodule_name);
        let output = std::process::Command::new("git")
            .args(["submodule", "add", repo_url, &submodule_path])
            .output()?;
        if !output.status.success() {
            eprintln!(
                "Failed to add {submodule_name} submodule: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    Ok(())
}

fn remove_lib_directories() -> Result<(), Error> {
    let lib_path = Path::new("./contracts/lib");

    if lib_path.exists() && lib_path.is_dir() {
        for entry in std::fs::read_dir(lib_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                println!("Removing directory: {:?}", path);
                std::fs::remove_dir_all(path)?;
            }
        }
        println!("All directories in ./contracts/lib have been removed.");
    } else {
        println!("The ./contracts/lib directory does not exist or is not a directory.");
        return Err(Error::GenerationFailed(anyhow::Error::msg(
            "Failed to remove ./contracts/lib/ directories",
        )));
    }

    Ok(())
}
