use gadget_std::fmt::Display;
use gadget_std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "std", derive(clap::ValueEnum))]
#[cfg_attr(feature = "std", clap(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum SupportedChains {
    #[default]
    LocalTestnet,
    LocalMainnet,
    Testnet,
    Mainnet,
}

impl FromStr for SupportedChains {
    type Err = String;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        match s {
            "local_testnet" => Ok(SupportedChains::LocalTestnet),
            "local_mainnet" => Ok(SupportedChains::LocalMainnet),
            "testnet" => Ok(SupportedChains::Testnet),
            "mainnet" => Ok(SupportedChains::Mainnet),
            _ => Err(format!("Invalid chain: {}", s)),
        }
    }
}

impl Display for SupportedChains {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SupportedChains::LocalTestnet => write!(f, "local_testnet"),
            SupportedChains::LocalMainnet => write!(f, "local_mainnet"),
            SupportedChains::Testnet => write!(f, "testnet"),
            SupportedChains::Mainnet => write!(f, "mainnet"),
        }
    }
}
