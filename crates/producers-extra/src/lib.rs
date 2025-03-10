//! Extra producers for the Blueprint SDK
//!
//! This is a catch-all crate for extra producers that are not protocol specific.
//!
//! ## Features
#![doc = document_features::document_features!()]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://cdn.prod.website-files.com/6494562b44a28080aafcbad4/65aaf8b0818b1d504cbdf81b_Tnt%20Logo.png"
)]

#[cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "cron")]
pub mod cron;
