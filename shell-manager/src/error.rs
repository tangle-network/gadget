use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

impl<T: Into<String>> From<T> for Error {
    fn from(message: String) -> Self {
        Self {
            message: message.into(),
        }
    }
}
