use std::path::PathBuf;

use sc_cli::{Error, SubstrateCli};

/// Key utilities for the cli.
#[derive(Debug, clap::Subcommand)]
pub enum DKGKeySubcommand {
    /// Prints all the information sotred int the DKG LocalKey.
    Inspect(InspectKeyCmd),
}

impl DKGKeySubcommand {
    /// run the key subcommands
    pub fn run<C: SubstrateCli>(&self, _cli: &C) -> Result<(), Error> {
        match self {
            DKGKeySubcommand::Inspect(cmd) => cmd.run(),
        }
    }
}

/// Prints all the information sotred int the DKG LocalKey.
#[derive(Debug, clap::Parser)]
#[clap(
name = "inspect-dkg-key",
about = "Load an encrypted DKG key from a file and the secret used to encrypt it from the secret-file"
)]
pub struct InspectKeyCmd {
    /// The path to the local key file.
    #[clap(long)]
    pub dkg_file: PathBuf,
    /// The path to the secret file.
    #[clap(long)]
    pub secret_file: PathBuf,
}

impl InspectKeyCmd {
    /// run the key subcommands
    pub fn run(&self) -> Result<(), Error> {
        Err(Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "not implemented")))
    }
}