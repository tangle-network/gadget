/// Job Definitions and Metadata
mod job_definition;
#[cfg(feature = "macros")]
pub mod macros;
/// Service Definition and Metadata
mod service;
/// Types used in the generation of the `blueprint.json` file
#[cfg(feature = "metadata-types")]
pub mod types;

pub use gadget_blueprint_serde::new_bounded_string;

pub use job_definition::{IntoJobDefinition, IntoJobMetadata, IntoTangleFieldTypes};
pub use service::IntoServiceMetadata;
