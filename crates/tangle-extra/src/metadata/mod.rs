/// Job Definitions and Metadata
mod job_definition;
/// Service Definition and Metadata
mod service;

pub use gadget_blueprint_serde::new_bounded_string;

pub use job_definition::{IntoJobDefinition, IntoJobMetadata, IntoTangleFieldTypes};
pub use service::IntoServiceMetadata;
