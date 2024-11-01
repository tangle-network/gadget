use alloc::string::{String, ToString};
use core::fmt::Display;
use core::fmt::Formatter;
use serde::{de, ser};

pub type Result<T> = core::result::Result<T, Error>;

/// Types that cannot be represented as a [`Field`](crate::Field)
///
/// Attempting to de/serialize any of these types will cause an error.
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum UnsupportedType {
    f32,
    f64,
    Map,
    /// Any enum that is not a simple unit enum
    ///
    /// ## Valid
    ///
    /// ```rust
    /// enum Foo {
    ///     Bar,
    ///     Baz,
    /// }
    /// ```
    ///
    /// ## Invalid
    ///
    /// ```rust
    /// enum Foo {
    ///     Bar(String),
    /// }
    /// ```
    ///
    /// Or
    ///
    /// ```rust
    /// enum Foo {
    ///     Baz { a: String },
    /// }
    /// ```
    NonUnitEnum,
}

#[derive(Debug)]
pub enum Error {
    UnsupportedType(UnsupportedType),
    /// Attempting to deserialize a [`char`] from a [`Field::String`](crate::Field::String)
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
            Error::BadCharLength(len) => {
                write!(f, "String contains {len} characters, expected 1")
            }
            Error::FromUtf8Error(e) => write!(f, "{e}"),
            Error::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl core::error::Error for Error {}
