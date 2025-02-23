use clap::{Args, ValueEnum};

#[derive(Debug, Clone, Args)]
pub struct CreateArgs {
    /// The name of the blueprint
    #[arg(short, long, value_name = "NAME", env = "NAME")]
    pub name: String,

    #[command(flatten)]
    pub blueprint_type: BlueprintType,
}

#[derive(Debug, Clone, Args, Default)]
#[group(required = false, multiple = false)]
pub struct BlueprintType {
    /// Create a Tangle blueprint
    #[arg(long, conflicts_with = "eigenlayer")]
    pub tangle: bool,

    /// Create an EigenLayer blueprint
    #[arg(long, value_name = "VARIANT", value_enum, num_args = 0..=1, default_value = "bls")]
    pub eigenlayer: Option<EigenlayerVariant>,
}

#[derive(Debug, Default, Clone, Copy, ValueEnum)]
pub enum EigenlayerVariant {
    #[default]
    BLS,
    ECDSA,
}

impl BlueprintType {
    pub fn get_type(&self) -> Option<BlueprintVariant> {
        if self.eigenlayer.is_some() {
            self.eigenlayer.map(BlueprintVariant::Eigenlayer)
        } else if self.tangle {
            Some(BlueprintVariant::Tangle)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum BlueprintVariant {
    Tangle,
    Eigenlayer(EigenlayerVariant),
}
