//! Tangle Network Blueprint Extra functionality

extern crate alloc;

/// Tangle Network Job Consumers
pub mod consumer;
/// Tangle Specific extractors
pub mod extract;
/// Tangle Specific filters
pub mod filters;
/// Tangle Specific layers
pub mod layers;
/// Tangle Blueprint Build Metadata
pub mod metadata;
/// Tangle Network Job Producers
pub mod producer;
pub mod util;

macro_rules! count {
    ($val:ident, $($rest:tt)*) => {
        1 + crate::count!($($rest)*)
    };
    ($val:ident) => {
        1
    };
    () => {
        0
    }
}

pub(crate) use count;
