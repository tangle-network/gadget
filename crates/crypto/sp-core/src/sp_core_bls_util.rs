use gadget_std::{
    string::{String, ToString},
    vec::Vec,
};
use sp_core::ByteArray;
use sp_core::Pair;

use crate::{impl_sp_core_key_type, impl_sp_core_pair_public};

/// Implements signature functionality for BLS signatures.
#[macro_export]
macro_rules! impl_sp_core_bls_signature {
    ($key_type:ident, $signature:ty) => {
        paste::paste! {
            #[derive(Clone, PartialEq, Eq)]
            pub struct [<$key_type Signature>](pub $signature);

            impl PartialOrd for [<$key_type Signature>] {
                fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                    let self_bytes: &[u8] = self.0.as_ref();
                    let other_bytes: &[u8] = other.0.as_ref();
                    self_bytes.partial_cmp(other_bytes)
                }
            }

            impl Ord for [<$key_type Signature>] {
                fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                    let self_bytes: &[u8] = self.0.as_ref();
                    let other_bytes: &[u8] = other.0.as_ref();
                    self_bytes.cmp(other_bytes)
                }
            }

            impl gadget_std::fmt::Debug for [<$key_type Signature>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    let bytes: &[u8] = self.0.as_ref();
                    write!(f, "{:?}", bytes)
                }
            }

            impl serde::Serialize for [<$key_type Signature>] {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    serializer.serialize_bytes(self.0.as_ref())
                }
            }

            impl<'de> serde::Deserialize<'de> for [<$key_type Signature>] {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    let bytes = <Vec<u8>>::deserialize(deserializer)?;
                    let sig = <$signature>::from_slice(&bytes)
                        .map_err(|_| serde::de::Error::custom("Invalid signature length"))?;
                    Ok([<$key_type Signature>](sig))
                }
            }
        }
    };
}

/// Implements both pair/public and signature traits for a given sp_core crypto type
macro_rules! impl_sp_core_bls_crypto {
    ($key_type:ident, $module:ident) => {
        impl_sp_core_pair_public!($key_type, sp_core::$module::Pair, sp_core::$module::Public);
        impl_sp_core_bls_signature!($key_type, sp_core::$module::Signature);
        impl_sp_core_key_type!($key_type, sp_core::$module::Pair);
    };
}

impl_sp_core_bls_crypto!(SpBls377, bls377);
impl_sp_core_bls_crypto!(SpBls381, bls381);
