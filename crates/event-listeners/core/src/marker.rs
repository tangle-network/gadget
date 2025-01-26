//! Marker Types
//!
//! These marker type are for internal use only. The help to differentiate between different
//! types of event listeners for the [`JobBuilder`](crate::job_runner::JobBuilder).
//! These are auto implemented for the respective event listeners in the macro code.
pub trait IsTangle {}
pub trait IsEvm {}
