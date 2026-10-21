use blueprint_runner::config::Protocol;
use cargo_tangle::command::keys::{
    SupportedKey, export_key, generate_key, generate_mnemonic, import_key, list_keys,
    prompt_for_keys,
};
use clap::Subcommand;
use color_eyre::eyre::{Result, eyre};
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(super) enum KeyCommands {
    /// Generate a new cryptographic key pair.
    ///
    /// Creates ECDSA (for signing transactions) or BLS keys (for threshold signatures).
    #[command(visible_alias = "g")]
    Generate {
        /// Key type: ecdsa (transactions), bls-bn254 (aggregation), sr25519, ed25519.
        #[arg(short = 't', long, value_enum)]
        key_type: SupportedKey,
        /// Output file path. If omitted, prints to stdout.
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
        /// Optional seed bytes for deterministic key generation.
        #[arg(long)]
        seed: Option<Vec<u8>>,
        /// Display the secret key (use with caution).
        #[arg(short = 'v', long)]
        show_secret: bool,
    },
    /// Import an existing key into the keystore.
    ///
    /// Use this to add a pre-existing private key to your local keystore.
    #[command(visible_alias = "i")]
    Import {
        /// Key type to import. Auto-detected if omitted.
        #[arg(short = 't', long, value_enum)]
        key_type: Option<SupportedKey>,
        /// Hex-encoded secret key (without 0x prefix).
        #[arg(short = 'x', long)]
        secret: Option<String>,
        /// Path to the keystore directory.
        #[arg(short = 'k', long)]
        keystore_path: PathBuf,
        /// Target protocol for key organization.
        #[arg(short = 'p', long, value_enum, default_value = "tangle")]
        protocol: Protocol,
    },
    /// Export a key from the keystore by its public key.
    #[command(visible_alias = "e")]
    Export {
        /// Key type to export.
        #[arg(short = 't', long, value_enum)]
        key_type: SupportedKey,
        /// Public key (hex) to look up in the keystore.
        #[arg(short = 'p', long)]
        public: String,
        /// Path to the keystore directory.
        #[arg(short = 'k', long)]
        keystore_path: PathBuf,
    },
    /// List all keys stored in the keystore.
    #[command(visible_alias = "l")]
    List {
        /// Path to the keystore directory.
        #[arg(short = 'k', long)]
        keystore_path: PathBuf,
    },
    /// Generate a new BIP-39 mnemonic phrase.
    ///
    /// Use this to create a seed phrase for deriving keys.
    #[command(visible_alias = "m")]
    GenerateMnemonic {
        /// Number of words (12, 15, 18, 21, or 24). Default: 12.
        #[arg(short = 'w', long, value_parser = clap::value_parser!(u32).range(12..=24))]
        word_count: Option<u32>,
    },
}

pub(super) fn execute(command: KeyCommands) -> Result<()> {
    match command {
        KeyCommands::Generate {
            key_type,
            output,
            seed,
            show_secret,
        } => {
            let seed = seed.map(hex::decode).transpose()?;
            let (public, secret) =
                generate_key(key_type, output.as_ref(), seed.as_deref(), show_secret)?;

            println!("Generated {key_type:?} key:");
            println!("Public key: {public}");
            if show_secret || output.is_none() {
                println!("Private key: {}", secret.expect("Missing secret"));
            }
        }
        KeyCommands::Import {
            key_type,
            secret,
            keystore_path,
            protocol,
        } => {
            if let Some(kind) = key_type {
                let secret = secret.ok_or_else(|| eyre!("Secret key is required"))?;
                let public = import_key(protocol, kind, &secret, &keystore_path)?;
                println!("Imported {kind:?} key:");
                println!("Public key: {public}");
            } else {
                let key_pairs = prompt_for_keys(vec![])?;
                for (kind, secret) in key_pairs {
                    let public = import_key(protocol, kind, &secret, &keystore_path)?;
                    println!("Imported {kind:?} key:");
                    println!("Public key: {public}");
                }
            }
        }
        KeyCommands::Export {
            key_type,
            public,
            keystore_path,
        } => {
            let secret = export_key(key_type, &public, &keystore_path)?;
            println!("Private key: {secret}");
        }
        KeyCommands::List { keystore_path } => {
            let keys = list_keys(&keystore_path)?;
            for (kind, public) in keys {
                println!("{kind:?}: {public}");
            }
        }
        KeyCommands::GenerateMnemonic { word_count } => {
            let mnemonic = generate_mnemonic(word_count)?;
            println!("Mnemonic: {mnemonic}");
        }
    }
    Ok(())
}
