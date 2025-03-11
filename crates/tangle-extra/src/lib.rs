//! Tangle Network Blueprint Extra functionality
//!
//! ## Features
#![doc = document_features::document_features!()]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://cdn.prod.website-files.com/6494562b44a28080aafcbad4/65aaf8b0818b1d504cbdf81b_Tnt%20Logo.png"
)]

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
#[cfg(any(feature = "std", feature = "web"))]
pub mod util;

pub use tangle_subxt::subxt_core;
