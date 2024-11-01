use core::fmt::Display;
use core::fmt::Formatter;
use serde::{de, ser};
use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::FieldType;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum UnsupportedType {
    f32,
    f64,
    Map,
    NonUnitEnum,
}

#[derive(Debug)]
pub enum Error {
    UnsupportedType(UnsupportedType),
    UnexpectedType {
        expected: FieldType,
        actual: Option<FieldType>,
    },
    BadCharLength(usize),
    FromUtf8Error(alloc::string::FromUtf8Error),
    Other(String),
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Other(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Other(msg.to_string())
    }
}

impl From<alloc::string::FromUtf8Error> for Error {
    fn from(err: alloc::string::FromUtf8Error) -> Self {
        Error::FromUtf8Error(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::UnsupportedType(unsupported_type) => {
                write!(f, "Type `{:?}` unsupported", unsupported_type)
            }
            Error::UnexpectedType { expected, actual } => {
                write!(
                    f,
                    "Expected field of type `{:?}`, got `{:?}`",
                    expected, actual
                )
            }
            Error::BadCharLength(len) => {
                write!(f, "String contains {len} characters, expected 1")
            }
            Error::FromUtf8Error(e) => write!(f, "{e}"),
            Error::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl core::error::Error for Error {}
