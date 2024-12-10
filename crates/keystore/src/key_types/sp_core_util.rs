use sp_core::ByteArray;
use sp_core::Pair;

/// Implements serde and KeyType trait for any sp_core crypto type.
///
/// Implements common functionality for key pairs and public keys.
#[macro_export]
macro_rules! impl_sp_core_pair_public {
    ($key_type:ident, $pair_type:ty, $public:ty) => {
        paste::paste! {
            /// Wrapper struct for the cryptographic key pair.
            ///
            /// This provides a safe interface for serialization and deserialization
            /// of the underlying key pair type.
            #[derive(Clone)]
            pub struct [<$key_type Pair>](pub $pair_type);

            impl PartialEq for [<$key_type Pair>] {
                fn eq(&self, other: &Self) -> bool {
                    self.0.to_raw_vec() == other.0.to_raw_vec()
                }
            }

            impl Eq for [<$key_type Pair>] {}

            impl PartialOrd for [<$key_type Pair>] {
                fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                    self.0.to_raw_vec().partial_cmp(&other.0.to_raw_vec())
                }
            }

            impl Ord for [<$key_type Pair>] {
                fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                    self.0.to_raw_vec().cmp(&other.0.to_raw_vec())
                }
            }

            impl gadget_std::fmt::Debug for [<$key_type Pair>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.0.to_raw_vec())
                }
            }

            impl serde::Serialize for [<$key_type Pair>] {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    let bytes = self.0.to_raw_vec();
                    serializer.serialize_bytes(&bytes)
                }
            }

            impl<'de> serde::Deserialize<'de> for [<$key_type Pair>] {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    let bytes = <Vec<u8>>::deserialize(deserializer)?;
                    let pair = <$pair_type>::from_seed_slice(&bytes)
                        .map_err(|e| serde::de::Error::custom(e.to_string()))?;
                    Ok([<$key_type Pair>](pair))
                }
            }

            /// Wrapper struct for the cryptographic public key.
            #[derive(Clone)]
            pub struct [<$key_type Public>](pub <$pair_type as sp_core::Pair>::Public);

            impl PartialEq for [<$key_type Public>]{
                fn eq(&self, other: &Self) -> bool {
                    self.0 == other.0
                }
            }

            impl Eq for [<$key_type Public>]{}

            impl PartialOrd for [<$key_type Public>]{
                fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                    self.0.to_raw_vec().partial_cmp(&other.0.to_raw_vec())
                }
            }

            impl Ord for [<$key_type Public>]{
                fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                    self.0.to_raw_vec().cmp(&other.0.to_raw_vec())
                }
            }

            impl gadget_std::fmt::Debug for [<$key_type Public>]{
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.0.to_raw_vec())
                }
            }

            impl serde::Serialize for [<$key_type Public>]{
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    let bytes = self.0.to_raw_vec();
                    serializer.serialize_bytes(&bytes)
                }
            }

            impl<'de> serde::Deserialize<'de> for [<$key_type Public>]{
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    let bytes = <Vec<u8>>::deserialize(deserializer)?;
                    let public = <$pair_type as sp_core::Pair>::Public::from_slice(&bytes)
                        .map_err(|_| serde::de::Error::custom("Invalid public key length"))?;
                    Ok([<$key_type Public>](public))
                }
            }
        }
    };
}

/// Implements signature functionality for non-BLS signatures.
#[macro_export]
macro_rules! impl_sp_core_signature {
    ($key_type:ident, $pair_type:ty) => {
        paste::paste! {
            #[derive(Clone, PartialEq, Eq)]
            pub struct [<$key_type Signature>](pub <$pair_type as sp_core::Pair>::Signature);

            impl PartialOrd for [<$key_type Signature>] {
                fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                    self.0.0.partial_cmp(&other.0.0)
                }
            }

            impl Ord for [<$key_type Signature>] {
                fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                    self.0.0.cmp(&other.0.0)
                }
            }

            impl gadget_std::fmt::Debug for [<$key_type Signature>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.0.0)
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
                    let sig = <$pair_type as sp_core::Pair>::Signature::from_slice(&bytes)
                        .map_err(|_| serde::de::Error::custom("Invalid signature length"))?;
                    Ok([<$key_type Signature>](sig))
                }
            }
        }
    };
}

/// Implements signature functionality for BLS signatures.
#[macro_export]
macro_rules! impl_sp_core_bls_signature {
    ($key_type:ident, $signature:ty) => {
        paste::paste! {
            #[derive(Clone, PartialEq, Eq)]
            pub struct [<$key_type Signature>](pub $signature);

            impl PartialOrd for [<$key_type Signature>] {
                fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                    self.0.as_ref().partial_cmp(other.0.as_ref())
                }
            }

            impl Ord for [<$key_type Signature>] {
                fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                    self.0.as_ref().cmp(other.0.as_ref())
                }
            }

            impl gadget_std::fmt::Debug for [<$key_type Signature>] {
                fn fmt(&self, f: &mut gadget_std::fmt::Formatter<'_>) -> gadget_std::fmt::Result {
                    write!(f, "{:?}", self.0.as_ref())
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
                        .ok_or_else(|| serde::de::Error::custom("Invalid signature length"))?;
                    Ok([<$key_type Signature>](sig))
                }
            }
        }
    };
}

/// Implements both pair/public and signature traits for a given sp_core crypto type
#[macro_export]
macro_rules! impl_sp_core_crypto {
    ($key_type:ident, $module:ident) => {
        impl_sp_core_pair_public!($key_type, sp_core::$module::Pair, sp_core::$module::Public);
        impl_sp_core_signature!($key_type, sp_core::$module::Pair);
    };
}

#[cfg(feature = "sp-core-bls")]
impl_sp_core_crypto!(SpBls377, bls377);
#[cfg(feature = "sp-core-bls")]
impl_sp_core_crypto!(SpBls381, bls381);
impl_sp_core_crypto!(SpEcdsa, ecdsa);
impl_sp_core_crypto!(SpEd25519, ed25519);
impl_sp_core_crypto!(SpSr25519, sr25519);
