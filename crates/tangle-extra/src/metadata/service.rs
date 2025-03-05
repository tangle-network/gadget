//! Service metadata traits and implementations for Tangle services.
//!
//! This module provides traits and implementations for converting Rust types into Tangle service
//! metadata that can be used when interacting with the Tangle Services pallet. The main trait is:
//!
//! - [`IntoServiceMetadata`]: Converts a type into service metadata
//!
//! # Examples
//!
//! ```
//! use gadget_blueprint_serde::new_bounded_string;
//! use blueprint_tangle_extra::metadata::IntoServiceMetadata;
//! use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::service::ServiceMetadata;
//!
//! struct MyService {
//!     name: String,
//!     description: Option<String>,
//!     author: Option<String>,
//! }
//!
//! impl IntoServiceMetadata for MyService {
//!     fn into_service_metadata(self) -> ServiceMetadata {
//!         // Convert the MyService instance into ServiceMetadata
//!         // Implementation details would go here
//!         ServiceMetadata {
//!             name: new_bounded_string(self.name),
//!             description: self.description.map(new_bounded_string),
//!             author: self.author.map(new_bounded_string),
//!             category: None,
//!             code_repository: None,
//!             logo: None,
//!             website: None,
//!             license: None,
//!         }
//!     }
//! }
//! ```

use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::service::ServiceMetadata;

/// Trait for types that can be converted into service metadata.
///
/// This trait should be implemented by types that represent service configuration
/// and can be converted to the Tangle [`ServiceMetadata`] format.
pub trait IntoServiceMetadata {
    /// Converts the implementor into service metadata.
    ///
    /// # Returns
    ///
    /// A [`ServiceMetadata`] instance containing metadata about the service.
    fn into_service_metadata(self) -> ServiceMetadata;
}
