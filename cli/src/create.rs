use crate::foundry::FoundryToolchain;
use clap::Args;
use gadget_sdk::tracing;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to generate blueprint: {0}")]
    GenerationFailed(anyhow::Error),
    #[error("Failed to initialize submodules, see .gitmodules to add them manually")]
    SubmoduleInit,
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

#[derive(Args, Debug, Clone, Default)]
#[group(id = "source", required = false, multiple = false)]
pub struct Source {
    #[command(flatten)]
    repo: Option<RepoArgs>,

    #[arg(short, long, group = "source")]
    path: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
#[group(requires = "repo")]
pub struct RepoArgs {
    #[arg(short, long, env, required = false, group = "source")]
    repo: String,
    #[arg(short, long, env)]
    branch: Option<String>,
    #[arg(short, long, env, conflicts_with = "branch")]
    tag: Option<String>,
}

impl From<Source> for Option<cargo_generate::TemplatePath> {
    fn from(value: Source) -> Self {
        let mut template_path = cargo_generate::TemplatePath::default();

        match value {
            Source {
                repo: Some(repo_args),
                ..
            } => {
                template_path.git = Some(repo_args.repo);
                template_path.branch = repo_args.branch;
                template_path.tag = repo_args.tag;
                Some(template_path)
            }
            Source {
                path: Some(path), ..
            } => {
                template_path.path = Some(path.to_string_lossy().into());
                Some(template_path)
            }
            Source {
                repo: None,
                path: None,
            } => None,
        }
    }
}

pub fn new_blueprint(name: String, source: Option<Source>) -> Result<(), Error> {
    println!("Generating blueprint with name: {}", name);

    let source = source.unwrap_or_default();
    let template_path_opt: Option<cargo_generate::TemplatePath> = source.into();

    let template_path = template_path_opt.unwrap_or_else(|| {
        // TODO: Interactive selection (#352)
        cargo_generate::TemplatePath {
            git: Some(String::from(
                "https://github.com/tangle-network/blueprint-template/",
            )),
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
