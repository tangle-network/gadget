//! Blueprint SDK Job Router
//!
//! This crate provides the [`Router`], which is used to route jobs to different handlers based on their [`JobId`].
//!
//! # Usage
//!
//! ```rust
//! use blueprint_sdk::Router;
//! use blueprint_sdk::job::JobWithoutContextExt;
//! use blueprint_sdk::{IntoJobResult, JobResult};
//!
//! // Define a job function. See [`Job`] for more details
//! async fn my_job() -> String {
//!     String::from("Hello world!")
//! }
//!
//! // Give the job some kind of ID. Many types can be converted
//! // into a [`JobId`]. See [`IntoJobId`] for more details
//! const MY_JOB_ID: u32 = 0;
//!
//! let router = Router::new().route(MY_JOB_ID, my_job);
//!
//! # let _: Router = router;
//! ```
//!
//! # See also
//!
//! - [`Job`](https://docs.rs/blueprint_sdk/latest/blueprint_sdk/trait.Job.html)
//! - [`JobId`]
//! - [`IntoJobId`](https://docs.rs/blueprint_sdk/latest/blueprint_sdk/trait.IntoJobId.html)
//!
//! [`JobId`]: https://docs.rs/blueprint_sdk/latest/blueprint_sdk/struct.JobId.html
//!
//! ## Features
#![doc = document_features::document_features!()]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://cdn.prod.website-files.com/6494562b44a28080aafcbad4/65aaf8b0818b1d504cbdf81b_Tnt%20Logo.png"
)]
#![no_std]

extern crate alloc;

#[doc(hidden)]
pub mod __private {
    #[cfg(feature = "tracing")]
    pub use blueprint_core::__private::tracing;
}

mod boxed;
pub mod future;
mod job_id_router;
pub mod routing;
mod util;

#[cfg(test)]
pub(crate) mod test_helpers;
#[cfg(test)]
mod tests;

pub use routing::Router;
