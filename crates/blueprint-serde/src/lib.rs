//! Compatibility crate for using the [Serde](https://docs.rs/serde) serialization frame with data from [`tangle_subxt`](https://docs.rs/tangle_subxt)
//!
//! This crate provides two primary functions:
//!
//! * [`to_field`] - Convert a [`Serialize`] type to a [`Field`]
//! * [`from_field`] - Convert a [`Field`] to a [`DeserializeOwned`] type
//!
//! # Examples
//!
//! ```rust
//! use gadget_blueprint_serde::{new_bounded_string, BoundedVec, Field};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(PartialEq, Debug, Serialize, Deserialize)]
//! struct Person {
//!     name: String,
//!     age: u8,
//! }
//!
//! let person = Person {
//!     name: String::from("John"),
//!     age: 40,
//! };
//!
//! let expected = Field::Struct(
//!     new_bounded_string("Person"),
//!     Box::new(BoundedVec(vec![
//!         (
//!             new_bounded_string("name"),
//!             Field::String(new_bounded_string("John")),
//!         ),
//!         (new_bounded_string("age"), Field::Uint8(40)),
//!     ])),
//! );
//!
//! // Convert our `Serialize` type to a `tangle_subxt::Field`
//! let field = gadget_blueprint_serde::to_field(&person).unwrap();
//! assert_eq!(expected, field);
//!
//! // Convert our `tangle_subxt::Field` back to a `Person`
//! let person_deserialized: Person = gadget_blueprint_serde::from_field(field).unwrap();
//! assert_eq!(person, person_deserialized);
//! ```

#![cfg_attr(feature = "std", no_std)]

mod de;
pub mod error;
mod ser;
#[cfg(test)]
mod tests;

extern crate alloc;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tangle_subxt::subxt_core::utils::AccountId32;
pub use tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::field::Field;
pub use tangle_subxt::tangle_testnet_runtime::api::runtime_types::bounded_collections::bounded_vec::BoundedVec;
pub use ser::new_bounded_string;
pub use serde_bytes::ByteBuf;
use error::Result;

/// Derive a [`Field`] from an instance of type `S`
///
/// # Errors
///
/// * Attempting to serialize an [`UnsupportedType`](error::UnsupportedType)
///
/// # Examples
///
/// ```rust
/// use gadget_blueprint_serde::{new_bounded_string, BoundedVec, Field};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Person {
///     name: String,
///     age: u8,
/// }
///
/// let person = Person {
///     name: String::from("John"),
///     age: 40,
/// };
///
/// let expected = Field::Struct(
///     new_bounded_string("Person"),
///     Box::new(BoundedVec(vec![
///         (
///             new_bounded_string("name"),
///             Field::String(new_bounded_string("John")),
///         ),
///         (new_bounded_string("age"), Field::Uint8(40)),
///     ])),
/// );
///
/// let field = gadget_blueprint_serde::to_field(person).unwrap();
/// assert_eq!(expected, field);
/// ```
pub fn to_field<S>(value: S) -> Result<Field<AccountId32>>
where
    S: Serialize,
{
    let mut ser = ser::Serializer;
    value.serialize(&mut ser)
}

/// Derive an instance of type `D` from a [`Field`]
///
/// # Errors
///
/// * Attempting to deserialize an [`UnsupportedType`](error::UnsupportedType)
/// * Attempting to deserialize non UTF-8 bytes into a [`String`](alloc::string::String)
/// * Any type mismatch (e.g. attempting to deserialize [`Field::Int8`] into a [`char`]).
///
/// # Examples
///
/// ```rust
/// use gadget_blueprint_serde::{new_bounded_string, BoundedVec, Field};
/// use serde::Deserialize;
///
/// #[derive(Deserialize, Debug)]
/// struct Person {
///     name: String,
///     age: u8,
/// }
///
/// let field = Field::Struct(
///     new_bounded_string("Person"),
///     Box::new(BoundedVec(vec![
///         (
///             new_bounded_string("name"),
///             Field::String(new_bounded_string("John")),
///         ),
///         (new_bounded_string("age"), Field::Uint8(40)),
///     ])),
/// );
///
/// let person: Person = gadget_blueprint_serde::from_field(field).unwrap();
/// println!("{:#?}", person);
/// ```
pub fn from_field<D>(field: Field<AccountId32>) -> Result<D>
where
    D: DeserializeOwned,
{
    let de = de::Deserializer(field);
    D::deserialize(de)
}
