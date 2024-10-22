use clap::Args;
use std::path::PathBuf;

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
    #[arg(short, long, required = false, group = "source")]
    repo: String,
    #[arg(short, long)]
    branch: Option<String>,
    #[arg(short, long, conflicts_with = "branch")]
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

pub fn new_blueprint(name: String, source: Option<Source>) {
    println!("Generating blueprint with name: {}", name);

    let source = source.unwrap_or_default();
    let template_path_opt: Option<cargo_generate::TemplatePath> = source.into();

    let template_path = match template_path_opt {
        Some(tp) => tp,
        None => {
            // TODO: Interactive selection (#352)
            cargo_generate::TemplatePath {
                git: Some(String::from(
                    "https://github.com/tangle-network/blueprint-template/",
                )),
                branch: Some(String::from("main")),
                ..Default::default()
            }
        }
    };

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
    });

    match path {
        Ok(path) => {
            println!("Blueprint generated at: {}", path.display());
        }
        Err(e) => {
            eprintln!("Error generating blueprint: {}", e);
        }
    }
}
