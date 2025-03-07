pub mod error;
pub mod manager;
pub(crate) mod types;
pub(crate) mod utils;
pub use error::Error;

#[cfg(test)]
mod tests;
