use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SubstrateGadgetError {}

impl Display for SubstrateGadgetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for SubstrateGadgetError {}
