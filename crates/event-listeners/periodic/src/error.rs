use gadget_std::string::String;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Inner listener error: {0}")]
    InnerListener(String),
}

pub type Result<T> = gadget_std::result::Result<T, Error>;
