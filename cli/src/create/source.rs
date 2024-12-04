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
