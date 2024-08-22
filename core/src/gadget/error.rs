use core::error::Error;
use core::fmt::{Debug, Display, Formatter};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GadgetError {
    FinalityNotificationStreamEnded,
    ProtocolMessageStreamEnded,
}

impl Display for GadgetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for GadgetError {}
