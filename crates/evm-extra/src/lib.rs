//! EVM Job Utilities
//!
//! This crate provides utilities for working with EVM-based job processing in the Blueprint framework.
//! It includes functionality for event extraction, block processing, finality determination,
//! and contract interaction.

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

pub mod extract;
pub mod filters;
pub mod producer;
