use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Shell Manager",
    about = "An MPC executor that connects to the Tangle network and runs protocols dynamically on the fly"
)]
pub struct ShellManagerOpts {
    /// The path to the shell configuration file
    #[structopt(parse(from_os_str), short = "s", long = "shell-config")]
    pub shell_config: PathBuf,
    #[structopt(parse(from_os_str), short = "p", long = "protocols-config")]
    pub protocols_config: PathBuf,
    /// The verbosity level, can be used multiple times
    #[structopt(long, short = "v", parse(from_occurrences))]
    pub verbose: i32,
    /// Whether to use pretty logging
    #[structopt(long)]
    pub pretty: bool,
    /// Wether this is debug mode or not
    #[structopt(long, short = "t")]
    pub test: bool,
}
