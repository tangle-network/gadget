pub use crate::create::error::Error;
pub use crate::create::source::Source;
pub use crate::create::types::BlueprintType;
use crate::foundry::FoundryToolchain;
use gadget_sdk::tracing;
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
    let contracts = path.join("contracts");
    if !contracts.exists() {
        return Ok(());
    }

    let foundry = FoundryToolchain::new();
    if !foundry.forge.is_installed() {
        tracing::warn!("Forge not installed, skipping dependencies");
        tracing::warn!("NOTE: See <https://getfoundry.sh>");
        tracing::warn!("NOTE: After installing Forge, you can run `forge soldeer update -d` to install dependencies");
        return Ok(());
    }

    std::env::set_current_dir(contracts)?;
    if let Err(e) = foundry.forge.install_dependencies() {
        tracing::error!("{e}");
    }

    Ok(())
}
