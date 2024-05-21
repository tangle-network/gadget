use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FinalityNotification {
    /// Finalized block number.
    pub number: u64,
    /// Finalized block header hash.
    pub hash: [u8; 32],
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SubstrateGadgetError {}

impl Display for SubstrateGadgetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for SubstrateGadgetError {}


