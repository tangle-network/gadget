#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "bls")]
mod bls;
#[cfg(feature = "bls")]
pub use bls::*;

#[cfg(feature = "aggregation")]
mod aggregation;

pub mod error;

#[cfg(test)]
mod tests;

use gadget_crypto_core::BytesEncoding;
use gadget_std::{string::String, vec::Vec};
use sp_core::{ByteArray, Pair};

/// Implements serde and KeyType trait for any sp_core crypto type.
///
/// Implements common functionality for key pairs and public keys.
macro_rules! impl_sp_core_pair_public {
    ($key_type:ident, $pair_type:ty, $public:ty) => {
        paste::paste! {
            /// Wrapper struct for the cryptographic key pair.
            ///
            /// This provides a safe interface for serialization and deserialization
            /// of the underlying key pair type.
            #[derive(Clone)]
            pub struct [<Sp $key_type Pair>](pub $pair_type);

            impl PartialEq for [<Sp $key_type Pair>] {
                fn eq(&self, other: &Self) -> bool {
                    self.to_bytes() == other.to_bytes()
                }
            }

            impl Eq for [<Sp $key_type Pair>] {}

            impl PartialOrd for [<Sp $key_type Pair>] {
                fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for [<Sp $key_type Pair>] {
                fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                    self.to_bytes().cmp(&other.to_bytes())
                }
            }

            impl gadget_std::fmt::Debug for [<Sp $key_type Pair>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.to_bytes())
                }
            }

            impl serde::Serialize for [<Sp $key_type Pair>] {
                fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    <Vec::<u8>>::serialize(&self.to_bytes(), serializer)
                }
            }

            impl<'de> serde::Deserialize<'de> for [<Sp $key_type Pair>] {
                fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    let seed = <Vec::<u8>>::deserialize(deserializer)?;
                    let pair = <$pair_type>::from_seed_slice(&seed).map_err(|_| serde::de::Error::custom("Invalid seed length"))?;
                    Ok([<Sp $key_type Pair>](pair))
                }
            }

            /// Wrapper struct for the cryptographic public key.
            #[derive(Clone, serde::Serialize, serde::Deserialize)]
            pub struct [<Sp $key_type Public>](pub <$pair_type as sp_core::Pair>::Public);

            impl gadget_std::hash::Hash for [<Sp $key_type Public>] {
                fn hash<H: gadget_std::hash::Hasher>(&self, state: &mut H) {
                    self.0.to_raw_vec().hash(state);
                }
            }

            impl BytesEncoding for [<Sp $key_type Public>] {
                fn to_bytes(&self) -> Vec<u8> {
                    self.0.to_raw_vec()
                }

                fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, serde::de::value::Error> {
                    Self::from_bytes_impl(bytes)
                }
            }

            impl PartialEq for [<Sp $key_type Public>]{
                fn eq(&self, other: &Self) -> bool {
                    self.0 == other.0
                }
            }

            impl Eq for [<Sp $key_type Public>]{}

            impl PartialOrd for [<Sp $key_type Public>]{
                fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for [<Sp $key_type Public>]{
                fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                    self.to_bytes().cmp(&other.to_bytes())
                }
            }

            impl gadget_std::fmt::Debug for [<Sp $key_type Public>]{
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.to_bytes())
                }
            }

            impl gadget_std::fmt::Display for [<Sp $key_type Public>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{}", hex::encode(self.to_bytes()))
                }
            }
        }
    };
}

/// Implements signature functionality for non-BLS signatures.
macro_rules! impl_sp_core_signature {
    ($key_type:ident, $pair_type:ty) => {
        paste::paste! {
            #[derive(Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
            pub struct [<Sp $key_type Signature>](pub <$pair_type as sp_core::Pair>::Signature);

            impl PartialOrd for [<Sp $key_type Signature>] {
                fn partial_cmp(&self, other: &Self) -> Option<gadget_std::cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for [<Sp $key_type Signature>] {
                fn cmp(&self, other: &Self) -> gadget_std::cmp::Ordering {
                    self.0.0.cmp(&other.0.0)
                }
            }

            impl gadget_std::fmt::Debug for [<Sp $key_type Signature>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.0.0)
                }
            }

            impl gadget_std::fmt::Display for [<Sp $key_type Signature>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{}", hex::encode(self.0.0))
                }
            }

            impl BytesEncoding for [<Sp $key_type Signature>] {
                fn to_bytes(&self) -> Vec<u8> {
                    self.0.to_raw_vec()
                }

                fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, serde::de::value::Error> {
                    match <$pair_type as sp_core::Pair>::Signature::from_slice(bytes) {
                        Ok(sig) => Ok([<Sp $key_type Signature>](sig)),
                        Err(_) => Err(serde::de::Error::custom("Invalid signature length")),
                    }
                }
            }
        }
    };
}

/// Implements KeyType trait for non-BLS signatures.
macro_rules! impl_sp_core_key_type {
    ($key_type:ident, $pair_type:ty) => {
        paste::paste! {
            #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
            pub struct [<Sp $key_type>];

            impl gadget_crypto_core::KeyType for [<Sp $key_type>] {
                type Public = [<Sp $key_type Public>];
                type Secret = [<Sp $key_type Pair>];
                type Signature = [<Sp $key_type Signature>];
                type Error = $crate::error::SpCoreError;

                fn key_type_id() -> gadget_crypto_core::KeyTypeId {
                    gadget_crypto_core::KeyTypeId::$key_type
                }

                fn generate_with_seed(seed: Option<&[u8]>) -> $crate::error::Result<Self::Secret> {
                    match seed {
                        Some(seed) => {
                            if seed.len() != 32 {
                                return Err($crate::error::SpCoreError::InvalidSeed("Invalid seed length".into()));
                            }
                            let seed_array: [u8; 32] = seed.try_into()
                                .map_err(|_| $crate::error::SpCoreError::InvalidSeed("Invalid seed length".into()))?;
                            let pair = <$pair_type>::from_seed(&seed_array);
                            Ok([<Sp $key_type Pair>](pair))
                        }
                        None => {
                            #[cfg(feature = "std")]
                            let (pair, _) = <$pair_type>::generate();
                            #[cfg(not(feature = "std"))]
                            let pair = {
                                use gadget_std::Rng;
                                let seed = Self::get_test_rng().gen::<[u8; 32]>();
                                <$pair_type>::from_seed(&seed)
                            };
                            Ok([<Sp $key_type Pair>](pair))
                        }
                    }
                }

                fn generate_with_string(secret: String) -> $crate::error::Result<Self::Secret> {
                    // For now, treat the string as a hex-encoded seed
                    let seed = hex::decode(&secret)
                        .map_err(|_| $crate::error::SpCoreError::InvalidSeed("Invalid hex string".into()))?;
                    if seed.len() != 32 {
                        return Err($crate::error::SpCoreError::InvalidSeed("Invalid seed length".into()));
                    }
                    let seed_array: [u8; 32] = seed.try_into()
                        .map_err(|_| $crate::error::SpCoreError::InvalidSeed("Invalid seed length".into()))?;
                    let pair = <$pair_type>::from_seed(&seed_array);
                    Ok([<Sp $key_type Pair>](pair))
                }

                fn public_from_secret(secret: &Self::Secret) -> Self::Public {
                    [<Sp $key_type Public>](secret.0.public())
                }

                fn sign_with_secret(
                    secret: &mut Self::Secret,
                    msg: &[u8],
                ) -> $crate::error::Result<Self::Signature> {
                    Ok([<Sp $key_type Signature>](secret.0.sign(msg)))
                }

                fn sign_with_secret_pre_hashed(
                    secret: &mut Self::Secret,
                    msg: &[u8; 32],
                ) -> $crate::error::Result<Self::Signature> {
                    Ok([<Sp $key_type Signature>](secret.0.sign(msg)))
                }

                fn verify(public: &Self::Public, msg: &[u8], signature: &Self::Signature) -> bool {
                    <$pair_type as sp_core::Pair>::verify(&signature.0, msg, &public.0)
                }
            }

            impl [<Sp $key_type Pair>] {
                pub fn public(&self) -> [<Sp $key_type Public>] {
                    [<Sp $key_type Public>](self.0.public())
                }
            }

            impl BytesEncoding for [<Sp $key_type Pair>] {
                fn to_bytes(&self) -> Vec<u8> {
                    self.0.to_raw_vec()
                }

                fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, serde::de::value::Error> {
                    let inner = <$pair_type>::from_seed_slice(bytes).map_err(|_| serde::de::Error::custom("Invalid seed length"))?;
                    Ok(Self(inner))
                }
            }

            impl gadget_std::ops::Deref for [<Sp $key_type Pair>] {
                type Target = $pair_type;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl gadget_std::ops::DerefMut for [<Sp $key_type Pair>] {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }
        }
    };
}

/// Implements both pair/public and signature traits for a given sp_core crypto type
macro_rules! impl_sp_core_crypto {
    ($key_type:ident, $module:ident) => {
        impl_sp_core_pair_public!($key_type, sp_core::$module::Pair, sp_core::$module::Public);
        impl_sp_core_signature!($key_type, sp_core::$module::Pair);
        impl_sp_core_key_type!($key_type, sp_core::$module::Pair);
    };
}

impl_sp_core_crypto!(Ecdsa, ecdsa);

impl SpEcdsaPublic {
    fn from_bytes_impl(bytes: &[u8]) -> Result<Self, serde::de::value::Error> {
        let inner = <sp_core::ecdsa::Pair as sp_core::Pair>::Public::from_full(bytes)
            .map_err(|_| serde::de::Error::custom("Invalid public key length"))?;
        Ok(Self(inner))
    }
}

impl_sp_core_crypto!(Ed25519, ed25519);

impl SpEd25519Public {
    fn from_bytes_impl(bytes: &[u8]) -> Result<Self, serde::de::value::Error> {
        let inner = <sp_core::ed25519::Pair as sp_core::Pair>::Public::from_slice(bytes)
            .map_err(|_| serde::de::Error::custom("Invalid public key length"))?;
        Ok(Self(inner))
    }
}

impl_sp_core_crypto!(Sr25519, sr25519);

impl SpSr25519Public {
    fn from_bytes_impl(bytes: &[u8]) -> Result<Self, serde::de::value::Error> {
        let inner = <sp_core::sr25519::Pair as sp_core::Pair>::Public::from_slice(bytes)
            .map_err(|_| serde::de::Error::custom("Invalid public key length"))?;
        Ok(Self(inner))
    }
}

impl Copy for SpEcdsaPublic {}
impl Copy for SpEd25519Public {}
impl Copy for SpSr25519Public {}
