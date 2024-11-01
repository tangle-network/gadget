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
use error::Result;

pub fn to_field<S>(value: &S) -> Result<Field<AccountId32>>
where
    S: ?Sized + Serialize,
{
    let mut ser = ser::Serializer;
    value.serialize(&mut ser)
}

pub fn from_field<D>(field: Field<AccountId32>) -> Result<D>
where
    D: DeserializeOwned,
{
    let de = de::Deserializer(field);
    D::deserialize(de)
}
