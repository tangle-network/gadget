//! EVM Job Utilities
//!
//! This crate provides utilities for working with EVM-based job processing in the Blueprint framework.
//! It includes functionality for event extraction, block processing, finality determination,
//! and contract interaction.
//!
//! ## Features
#![doc = document_features::document_features!()]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://cdn.prod.website-files.com/6494562b44a28080aafcbad4/65aaf8b0818b1d504cbdf81b_Tnt%20Logo.png"
)]
#![warn(missing_docs)]
#![deny(
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    missing_docs
)]

extern crate alloc;

pub mod consumer;
pub mod extract;
pub mod filters;
pub mod producer;
