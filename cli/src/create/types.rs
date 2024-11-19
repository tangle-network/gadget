use clap::{Args, ValueEnum};

#[derive(Debug, Clone, Args)]
pub struct CreateArgs {
    /// The name of the blueprint
    #[arg(short, long, value_name = "NAME", env = "NAME")]
    pub name: String,

    #[command(flatten)]
    pub blueprint_type: BlueprintType,
}

#[derive(Debug, Clone, Args)]
#[group(required = false, multiple = false)]
pub struct BlueprintType {
    /// Create a Tangle blueprint
    #[arg(long)]
    pub tangle: bool,

    /// Create an EigenLayer blueprint
    #[arg(long, value_name = "VARIANT", value_enum)]
    pub eigenlayer: Option<EigenlayerVariant>,
}

impl Default for BlueprintType {
    fn default() -> Self {
        Self {
            tangle: true,
            eigenlayer: None,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum EigenlayerVariant {
    BLS,
    ECDSA,
}

impl BlueprintType {
    pub fn get_type(&self) -> Option<BlueprintVariant> {
        if self.tangle {
            Some(BlueprintVariant::Tangle)
        } else {
            self.eigenlayer.map(BlueprintVariant::Eigenlayer)
        }
    }
}

#[derive(Debug, Clone)]
pub enum BlueprintVariant {
    Tangle,
    Eigenlayer(EigenlayerVariant),
}