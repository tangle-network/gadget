use gadget_std::string::String;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PeriodicEventListenerError {
    #[error("Inner listener error: {0}")]
    InnerListener(String),
}

pub type Result<T> = gadget_std::result::Result<T, PeriodicEventListenerError>;
